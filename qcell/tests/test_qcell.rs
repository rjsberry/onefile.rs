use std::panic;
use std::sync::atomic::{AtomicBool, Ordering::*};
use std::sync::Arc;
use std::thread;

use qcell::DoubleBufferedCell;

#[cfg(miri)]
const ITER: usize = 256;
#[cfg(not(miri))]
const ITER: usize = 1024 * 1024;

#[derive(Default)]
struct Exit(AtomicBool);

impl Exit {
    fn should_exit(&self) -> bool {
        self.0.load(Relaxed)
    }

    fn exit(&self) {
        self.0.store(true, Relaxed);
    }
}

impl Drop for Exit {
    fn drop(&mut self) {
        self.0.store(true, Relaxed);
    }
}

#[test]
fn write_uncontended_data_race() {
    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    struct Dummy([usize; 8]);

    impl Dummy {
        const A: Self = Self([!0, !0, !0, !0, 0, 0, 0, 0]);
        const B: Self = Self([0, 0, 0, 0, !0, !0, !0, !0]);
    }

    let cell = Arc::new(DoubleBufferedCell::new(Dummy::A));
    let exit = Arc::new(Exit::default());

    let cell2 = Arc::clone(&cell);
    let exit2 = Arc::clone(&exit);

    thread::spawn(move || {
        while !exit2.should_exit() {
            unsafe {
                cell2.write_uncontended(&Dummy::A);
                thread::yield_now();
                cell2.write_uncontended(&Dummy::B);
                thread::yield_now();
            }
        }
    });

    let mut a = 0;
    let mut b = 0;

    for _ in 0..ITER {
        match cell.read() {
            Dummy::A => a += 1,
            Dummy::B => b += 1,
            other => panic!("{:X?}", other),
        }
        thread::yield_now();
    }

    assert_eq!(a + b, ITER);
    assert!(a > ITER / 4, "a={}", a);
    assert!(b > ITER / 4, "b={}", b);

    exit.exit();
}

#[test]
fn write_uncontended_monotonicity() {
    let cell = Arc::new(DoubleBufferedCell::new(0_usize));
    let exit = Arc::new(Exit::default());

    let cell2 = Arc::clone(&cell);
    let exit2 = Arc::clone(&exit);

    thread::spawn(move || {
        let mut i = 1;
        while !exit2.should_exit() {
            unsafe {
                cell2.write_uncontended(&i);
            }
            i = i.saturating_add(1);
        }
    });

    let mut prev = 0;
    for _ in 0..ITER {
        let next = cell.read();
        assert!(next >= prev, "next={}, prev={}", next, prev);
        prev = next;
    }

    exit.exit();
}

#[test]
fn write_uncontended_concurrent_readers() {
    let cell = DoubleBufferedCell::new(0_usize);

    thread::scope(|s| {
        for _ in 0..8 {
            s.spawn(|| {
                while cell.read() != ITER {
                    thread::yield_now();
                }
            });
        }
        s.spawn(|| unsafe {
            for i in 0..=ITER {
                cell.write_uncontended(&i);
                thread::yield_now();
            }
        });
    });
}
