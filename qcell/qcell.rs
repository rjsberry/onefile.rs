// Copyright (C) 2022 by Richard Berry <rjsberry@proton.me>
//
// Permission to use, copy, modify, and/or distribute this software for any
// purpose with or without fee is hereby granted.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
// ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
// OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

//! Thread-safe lock-free synchronisation primitives built on atomics.
//!
//! The cells in this crate are `no_std`, with no allocations and no runtime
//! panics. They are designed as a mechanism to share data between high/low
//! priority tasks and ISRs when you do not specifically need to queue items.
//!
//! Types that are shared must be `Copy`; the cells double buffer the data
//! and safely synchronise `memcpy` access to the inner pointers.
//!
//! # Write Prioritisation
//!
//! Cells with _"write prioritisation"_ ensure that writers have wait-free
//! write access to the inner data as long as they are not preempted. This
//! makes them ideal for busing data from interrupts and high-priority task
//! to the rest of your application:
//!
//! ```
//! # #[derive(Copy, Clone)]
//! # struct State;
//! # impl State {
//! #     const fn empty() -> Self { Self }
//! #     fn record_from_isr() -> Self { Self }
//! # }
//! # fn task_logic(_: &State) {}
//! use qcell::DoubleBufferedCell;
//! 
//! static CELL: DoubleBufferedCell<State> = DoubleBufferedCell::new(State::empty());
//! 
//! unsafe extern "C" fn isr() {
//!     // the interrupt is the only writer of the state - it can write to the
//!     // cell at the same time we are reading from it elsewhere
//!     CELL.write_uncontended(&State::record_from_isr());
//! }
//! 
//! fn task(_: *mut ()) -> ! {
//!     loop {
//!         // if we get interrupted during the read, we'll end up with the
//!         // previous copy of the state until the next loop
//!         task_logic(&CELL.read());
//!     }    
//! }
//! ```

#![no_std]

use core::cell::UnsafeCell;
use core::hint;
use core::mem::MaybeUninit;
use core::ptr;

#[cfg(feature = "atomic_polyfill")]
use atomic_polyfill::{AtomicUsize, Ordering::*};
#[cfg(not(feature = "atomic_polyfill"))]
use core::sync::atomic::{AtomicUsize, Ordering::*};

use self::{bits::*, Slot::*};

/// A synchronised cell for concurrent task communication.
pub struct DoubleBufferedCell<T> {
    flags: AtomicUsize,
    slots: [UnsafeCell<T>; 2],
}

#[rustfmt::skip]
mod bits {
    // writer flags
    //
    // w1/w2 signal when a slot is being written to
    pub const WMASK: usize   = 0x0003;
    pub const WSH: usize     = 0x0000;
    pub const W1: usize      = 0x0001;
    pub const W2: usize      = 0x0002;

    // reader flags
    //
    // r1/r2 signal when a slot is being read from
    //
    // the bits behind rcmask are the number of currently active readers
    pub const RMASK: usize   = 0x000C;
    pub const RSH: usize     = 0x0002;
    pub const R1: usize      = 0x0004;
    pub const R2: usize      = 0x0008;
    pub const RCMASK: usize  = 0x7FC0;
    pub const RCSH: usize    = 0x0006;

    // priority flags
    //
    // p1 signals to attempt reads from slot 1 and writes to slot 2
    // p2 signals to attempt reads from slot 2 and writes to slot 1
    pub const PMASK: usize   = 0x0030;
    pub const PSH: usize     = 0x0004;
    pub const P1: usize      = 0x0010;
    pub const P2: usize      = 0x0020;

    // backoff flag
    //
    // do new readers need to spin-loop before reading a slot?
    pub const BACKOFF: usize = 0x8000;

    // combined flags
    pub const W1P1: usize    = W1 | P1;
    pub const W1P2: usize    = W1 | P2;
    pub const W1R2P1: usize  = W1 | R2 | P1;
    pub const W1R2P2: usize  = W1 | R2 | P2;
    pub const W2P1: usize    = W2 | P1;
    pub const W2P2: usize    = W2 | P2;
    pub const W2R1P1: usize  = W2 | R1 | P1;
    pub const W2R1P2: usize  = W2 | R1 | P2;
    pub const R1P1: usize    = R1 | P1;
    pub const R1P2: usize    = R1 | P2;
    pub const R2P1: usize    = R2 | P1;
    pub const R2P2: usize    = R2 | P2;
}

#[derive(Debug, Copy, Clone)]
#[repr(usize)]
enum Slot {
    Slot1,
    Slot2,
}

// impl DoubleBufferedCell

unsafe impl<T: Copy + Send> Sync for DoubleBufferedCell<T> {}

impl<T: Copy> DoubleBufferedCell<T> {
    /// Creates a new cell with an initial value.
    pub const fn new(init: T) -> Self {
        Self {
            flags: AtomicUsize::new(P2),
            slots: [UnsafeCell::new(init), UnsafeCell::new(init)],
        }
    }

    /// Reads the most recent value written to the cell.
    ///
    /// This function _might_ sit in a CAS busy-loop for short periods if
    /// there are a large number of concurrent readers. This ensures that
    /// barraging the cell with read operations does not cause it to get
    /// stuck in a state where it only reads stale data.
    pub fn read(&self) -> T {
        let mut slot = MaybeUninit::uninit();

        while self
            .flags
            .fetch_update(Acquire, Relaxed, |mut b| {
                debug_assert_ne!(
                    b & RMASK,
                    RMASK,
                    "[bug] :: readers occupying both buffer slots",
                );

                let num_rdrs = (b & RCMASK) >> RCSH;
                debug_assert_ne!(
                    num_rdrs,
                    RCMASK >> RCSH,
                    "[safety contract violation] :: too many concurrent readers",
                );

                // hold off on starting a read for now
                //
                // too many readers are hammering the cell and causing stale
                // data to continually be pumped out
                let backoff = (b & BACKOFF) != 0;
                if backoff && num_rdrs > 0 {
                    return None;
                }

                let mut b_next = move |rdr| {
                    b = (b & !RCMASK) | rdr | (num_rdrs + 1) << RCSH;
                    if backoff {
                        b &= !BACKOFF;
                    }
                    b
                };

                let (slot_choice, b_new) = match b & (WMASK | RMASK | PMASK) {
                    W1P1 | W2P2 if backoff => return None,
                    P1 | R1P1 | R1P2 | W2P1 | W2P2 | W2R1P1 | W2R1P2 => (Slot1, b_next(R1)),
                    P2 | R2P1 | R2P2 | W1P1 | W1P2 | W1R2P1 | W1R2P2 => (Slot2, b_next(R2)),
                    _ => {
                        debug_assert!(false, "[bug] :: invalid state (0x{:02x})", b);
                        // safety: api guarantees we don't see invalid state
                        unsafe {
                            hint::unreachable_unchecked();
                        }
                    }
                };

                // safety: `slot` is a valid ptr in local scope
                unsafe {
                    ptr::write(slot.as_mut_ptr(), slot_choice);
                }

                Some(b_new)
            })
            .is_err()
        {
            hint::spin_loop();
        }

        // safety: we've initialized `slot` if we've left the spin-loop
        let slot = unsafe { slot.assume_init() };
        // safety: `slot` as a `usize` can only be either 0 or 1
        let cell = unsafe { self.slots.get_unchecked(slot as usize) };
        // safety: api guarantees we have (possibly shared) read lock on pointer
        let val = unsafe { ptr::read_volatile(cell.get()) };

        let _ = self.flags.fetch_update(Release, Relaxed, |mut b| {
            let num_rdrs = (b & RCMASK) >> RCSH;
            if num_rdrs == 1 {
                b &= !((slot as usize + 1) << RSH);
            }
            Some((b & !RCMASK) | (num_rdrs - 1) << RCSH)
        });

        val
    }

    /// Writes a value to the cell without waiting.
    ///
    /// **Note:** Preempting an uncontended write may cause the operation
    /// to retry once it resumes.
    ///
    /// # Safety
    ///
    /// There can be at most one writer to the cell. It is a contract
    /// violation to write to the cell concurrently (e.g., from multiple
    /// preemptible tasks).
    ///
    /// It is safe to write to the cell at the same time others are reading
    /// from it. For example, this function can be used within an ISR to
    /// communicate data to lower priority tasks. A single cell may also be
    /// written to from multiple ISRs if those ISRs are not nested (i.e., they
    /// do not interrupt each other).
    pub unsafe fn write_uncontended(&self, value: &T) {
        let mut slot = MaybeUninit::uninit();

        let _ = self.flags.fetch_update(Acquire, Relaxed, |b| {
            debug_assert_eq!(
                b & WMASK,
                0,
                "[safety contract violation] :: multiple concurrent writers",
            );

            let (slot_choice, b_new) = match b & (RMASK | PMASK) {
                P2 | R2P2 => (Slot1, b | W1),
                R2P1 => (Slot1, b | W1 | BACKOFF),
                P1 | R1P1 => (Slot2, b | W2),
                R1P2 => (Slot2, b | W2 | BACKOFF),
                _ => {
                    debug_assert!(false, "[bug] :: invalid state (0x{:02x})", b);
                    // safety: api guarantees we don't see invalid state
                    hint::unreachable_unchecked();
                }
            };

            // safety: `slot` is a valid ptr in local scope
            ptr::write(slot.as_mut_ptr(), slot_choice);

            Some(b_new)
        });

        // safety: fetch update always initializes `slot`
        let slot = slot.assume_init();
        // safety: `slot` as a `usize` can only be either 0 or 1
        let cell = self.slots.get_unchecked(slot as usize);
        // safety: api guarantees we have write lock on pointer
        ptr::write_volatile(cell.get(), *value);

        let _ = self.flags.fetch_update(Release, Relaxed, |mut b| {
            debug_assert_eq!(b & WMASK, slot as usize + 1);
            b &= !((slot as usize + 1) << WSH);
            b &= !PMASK;
            b |= (slot as usize + 1) << PSH;
            Some(b)
        });
    }
}
