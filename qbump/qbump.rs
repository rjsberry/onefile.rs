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

//! Bump allocators compatible with static data buffers.
//!
//! The allocators in this crate are designed for applications where heap
//! allocation is generally forbidden or unused. It is recommended to use
//! these allocators with fallible allocation APIs to gracefully handle
//! allocations within your application.
//!
//! General usage would be allocating large amounts of objects in different
//! stages/sections of your application. Once all existing allocations have
//! been deallocated, the allocators will reset and be able to re-use their
//! original buffer.
//!
//! # Usage
//!
//! Stack buffer:
//!
//! ```
//! #![feature(allocator_api)]
//!
//! use qbump::{static_buf, Bump};
//!
//! let mut buf = [0; 128];
//! let bump = Bump::new(&mut buf);
//! let f: Box<dyn Fn() -> i32, &Bump> = Box::try_new_in(|| 123, &bump).unwrap();
//! assert_eq!(f(), 123);
//! ```
//!
//! Static buffer:
//!
//! ```
//! #![feature(allocator_api)]
//!
//! use qbump::{static_buf, Bump};
//!
//! let bump = Bump::new(static_buf!([u8; 128]));
//! let f: Box<dyn Fn() -> i32, &Bump> = Box::try_new_in(|| 123, &bump).unwrap();
//! assert_eq!(f(), 123);
//! ```
//!
//! Allocator re-use:
//!
//! ```
//! #![feature(allocator_api)]
//!
//! use qbump::{static_buf, Bump};
//!
//! let bump = Bump::new(static_buf!([u8; 1]));
//! let b: Box<u8, &Bump> = Box::try_new_in(1, &bump).unwrap();
//! assert!(Box::try_new_in(2, &bump).is_err());
//! drop(b);
//! let b: Box<u8, &Bump> = Box::try_new_in(2, &bump).unwrap();
//! ```

#![no_std]
#![feature(allocator_api)]
#![feature(nonnull_slice_from_raw_parts)]
#![feature(strict_provenance)]

extern crate alloc;

use core::cell::Cell;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ptr::{self, NonNull};
use core::sync::atomic::{self, AtomicPtr, AtomicUsize, Ordering::*};

use alloc::alloc::{AllocError, Allocator, Layout};

/// A single threaded bump allocator.
pub struct Bump<'a> {
    lower: *mut u8,
    upper: *mut u8,
    head: Cell<*mut u8>,
    count: Cell<usize>,

    _marker: PhantomData<&'a ()>,
}

/// A thread-safe atomic bump allocator.
pub struct AtomicBump<'a> {
    lower: *mut u8,
    upper: *mut u8,
    head: AtomicPtr<u8>,
    count: AtomicUsize,

    _marker: PhantomData<&'a ()>,
}

/// Safely return a reference to a static mutable buffer.
///
/// ```ignore
/// let buf = static_buf! {
///     // customize link section with meta attributes:
///     #[link_section = ".custom_section"]
///     [u8; 1024]
/// };
/// ```
#[macro_export]
macro_rules! static_buf {
    ($(#[$m:meta])* [u8; $len:literal]) => {{
        #[repr(transparent)]
        struct RacyCell(::core::cell::UnsafeCell<[u8; $len]>);
        unsafe impl Sync for RacyCell {}

        $(#[$m])*
        static BUF: RacyCell = RacyCell(::core::cell::UnsafeCell::new([0; $len]));
        static USE: ::core::sync::atomic::AtomicBool = ::core::sync::atomic::AtomicBool::new(false);

        if !USE.swap(true, ::core::sync::atomic::Ordering::Relaxed) {
            unsafe { &mut *BUF.0.get() as &mut [_] }
        } else {
            &mut [][..]
        }
    }}
}

// impl Bump

impl<'a> Bump<'a> {
    /// Creates a new bump allocator backed by a given buffer.
    pub fn new(buf: &'a mut [u8]) -> Self {
        unsafe { Self::from_ptr(buf.as_mut_ptr(), buf.len()) }
    }

    /// How many allocations has this allocator created?
    ///
    /// Once all buffers served by the allocator are deallocated the
    /// count will return to 0.
    #[inline]
    pub fn count(&self) -> usize {
        self.count.get()
    }
}

impl Bump<'_> {
    /// Creates a new bump allocator backed by a given buffer.
    ///
    /// # Safety
    ///
    /// Behaviour is undefined if any of the following are true:
    ///
    /// * `buf` must be valid for reads and writes of `len` bytes.
    /// * `buf` must be a single contiguous allocation.
    /// * The memory pointed to by `buf` must not be accessed by any
    ///   other means whilst the bump allocator owns it.
    pub unsafe fn from_ptr(buf: *mut u8, len: usize) -> Self {
        let lower = buf;
        let upper = lower.add(len);
        Self {
            lower,
            upper,
            head: Cell::new(upper),
            count: Cell::new(0),
            _marker: PhantomData,
        }
    }
}

unsafe impl Allocator for Bump<'_> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        if layout.size() == 0 {
            return Ok(NonNull::slice_from_raw_parts(NonNull::dangling(), 0));
        }

        let head = self.head.get();
        let new_head = head.with_addr(
            head.addr().checked_sub(layout.size()).ok_or(AllocError)? & !(layout.align() - 1),
        );

        if new_head.addr() < self.lower.addr() {
            // oom
            return Err(AllocError);
        }

        self.head.set(new_head);
        self.count.set(self.count.get() + 1);

        Ok(NonNull::slice_from_raw_parts(
            unsafe { NonNull::new_unchecked(new_head) },
            layout.size(),
        ))
    }

    unsafe fn deallocate(&self, _: NonNull<u8>, layout: Layout) {
        if layout.size() > 0 {
            let count = self.count.get();
            debug_assert!(count > 0);
            self.count.set(count - 1);
            if count == 1 {
                self.head.set(self.upper);
            }
        }
    }
}

// impl AtomicBump

unsafe impl Sync for AtomicBump<'_> {}

impl<'a> AtomicBump<'a> {
    /// Creates a new atomic bump allocator backed by a given buffer.
    pub fn new(buf: &'a mut [u8]) -> Self {
        unsafe { Self::from_ptr(buf.as_mut_ptr(), buf.len()) }
    }

    /// How many allocations has this allocator created?
    ///
    /// Once all buffers served by the allocator are deallocated the
    /// count will return to 0.
    #[inline]
    pub fn count(&self) -> usize {
        self.count.load(Relaxed)
    }
}

impl AtomicBump<'_> {
    /// Creates a new atomic bump allocator backed by a given buffer.
    ///
    /// # Safety
    ///
    /// Behaviour is undefined if any of the following are true:
    ///
    /// * `buf` must be valid for reads and writes of `len` bytes.
    /// * `buf` must be a single contiguous allocation.
    /// * The memory pointed to by `buf` must not be accessed by any
    ///   other means whilst the bump allocator owns it.
    pub const unsafe fn from_ptr(buf: *mut u8, len: usize) -> Self {
        let lower = buf;
        let upper = lower.add(len);
        Self {
            lower,
            upper,
            head: AtomicPtr::new(upper),
            count: AtomicUsize::new(0),
            _marker: PhantomData,
        }
    }
}

unsafe impl Allocator for AtomicBump<'_> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        if layout.size() == 0 {
            return Ok(NonNull::slice_from_raw_parts(NonNull::dangling(), 0));
        }

        let mut ptr = MaybeUninit::uninit();

        if self
            .head
            .fetch_update(Acquire, Relaxed, |head| {
                match head
                    .addr()
                    .checked_sub(layout.size())
                    .map(|unaligned| head.with_addr(unaligned & !(layout.align() - 1)))
                    .filter(|new_head| new_head.addr() >= self.lower.addr())
                {
                    // safety: `ptr` is a valid pointer in local scope
                    Some(addr) => unsafe {
                        ptr::write(ptr.as_mut_ptr(), addr);
                        Some(addr)
                    },
                    None => None,
                }
            })
            .is_err()
        {
            // oom
            return Err(AllocError);
        }

        self.count.fetch_add(1, Relaxed);

        Ok(NonNull::slice_from_raw_parts(
            unsafe { NonNull::new_unchecked(ptr.assume_init()) },
            layout.size(),
        ))
    }

    unsafe fn deallocate(&self, _: NonNull<u8>, layout: Layout) {
        if layout.size() > 0 {
            if self.count.fetch_sub(1, Release) == 1 {
                atomic::fence(Acquire);
                self.head.store(self.upper, Release);
            }
        }
    }
}
