#![feature(allocator_api)]

use std::mem;
use std::sync::Barrier;
use std::thread;

use qbump::{static_buf, AtomicBump, Bump};

macro_rules! aligned_buf {
    ($len:literal, $align:literal) => {{
        #[repr(align($align))]
        struct Buf([u8; $len]);

        impl ::std::ops::Deref for Buf {
            type Target = [u8];
            fn deref(&self) -> &[u8] {
                &self.0
            }
        }

        impl std::ops::DerefMut for Buf {
            fn deref_mut(&mut self) -> &mut [u8] {
                &mut self.0
            }
        }

        Buf([0; $len])
    }};
}

#[test]
fn empty_bump() {
    let bump = Bump::new(&mut []);
    assert!(Box::try_new_in(0_i32, &bump).is_err());
}

#[test]
fn bump_zst() {
    let bump = Bump::new(&mut []);
    let _zst = Box::try_new_in((), &bump).unwrap();
}

#[test]
fn bump_align_1() {
    let mut buf = aligned_buf!(1, 1);
    let bump = Bump::new(&mut buf);
    let ptr = Box::into_raw(Box::try_new_in(0_u8, &bump).unwrap());
    assert_eq!(ptr.align_offset(mem::align_of::<u8>()), 0);
}

#[test]
fn bump_align_2() {
    let mut buf = aligned_buf!(4, 2);
    let bump = Bump::new(&mut buf);
    let _ = Box::into_raw(Box::try_new_in(0_u8, &bump).unwrap());
    let ptr = Box::into_raw(Box::try_new_in(0_u16, &bump).unwrap());
    assert_eq!(ptr.align_offset(mem::align_of::<u16>()), 0);
}

#[test]
fn bump_align_4() {
    let mut buf = aligned_buf!(8, 4);
    let bump = Bump::new(&mut buf);
    let _ = Box::into_raw(Box::try_new_in(0_u8, &bump).unwrap());
    let ptr = Box::into_raw(Box::try_new_in(0_u32, &bump).unwrap());
    assert_eq!(ptr.align_offset(mem::align_of::<u32>()), 0);
}

#[test]
fn bump_align_8() {
    let mut buf = aligned_buf!(16, 8);
    let bump = Bump::new(&mut buf);
    let _ = Box::into_raw(Box::try_new_in(0_u8, &bump).unwrap());
    let ptr = Box::into_raw(Box::try_new_in(0_u64, &bump).unwrap());
    assert_eq!(ptr.align_offset(mem::align_of::<u64>()), 0);
}

#[test]
fn bump_align_16() {
    let mut buf = aligned_buf!(32, 16);
    let bump = Bump::new(&mut buf);
    let _ = Box::into_raw(Box::try_new_in(0_u8, &bump).unwrap());
    let ptr = Box::into_raw(Box::try_new_in(0_u128, &bump).unwrap());
    assert_eq!(ptr.align_offset(mem::align_of::<u128>()), 0);
}

#[test]
fn bump_drop_one() {
    let mut buf = aligned_buf!(4, 4);
    let bump = Bump::new(&mut buf);
    let ptr = Box::try_new_in(0_u32, &bump).unwrap();
    assert!(Box::try_new_in(0_u32, &bump).is_err());
    drop(ptr);
    assert!(Box::try_new_in(0_u32, &bump).is_ok());
}

#[test]
fn bump_drop_many() {
    let mut buf = aligned_buf!(12, 4);
    let bump = Bump::new(&mut buf);
    let ptr1 = Box::try_new_in(0_u32, &bump).unwrap();
    let ptr2 = Box::try_new_in(0_u32, &bump).unwrap();
    let ptr3 = Box::try_new_in(0_u32, &bump).unwrap();
    assert!(Box::try_new_in(0_u32, &bump).is_err());
    drop(ptr3);
    assert!(Box::try_new_in(0_u32, &bump).is_err());
    drop(ptr2);
    assert!(Box::try_new_in(0_u32, &bump).is_err());
    drop(ptr1);
    assert!(Box::try_new_in(0_u32, &bump).is_ok());
}

#[test]
#[rustfmt::skip]
fn bump_dyn() {
    trait V { fn v(&self) -> i32; }
    struct W(i32);
    impl V for W { fn v(&self) -> i32 { self.0 } }

    let mut buf = aligned_buf!(4, 4);
    let bump = Bump::new(&mut buf);
    let v: Box<dyn V, &Bump> = Box::try_new_in(W(123), &bump).unwrap();
    assert_eq!(v.v(), 123);
}

#[test]
fn static_bump() {
    let bump = Bump::new(static_buf!([u8; 8]));
    let ptr = Box::try_new_in(123_i32, &bump).unwrap();
    assert_eq!(*ptr, 123);
}

#[test]
fn concurrent_atomic_bump() {
    #[cfg(not(miri))]
    const N: usize = 1024;
    #[cfg(miri)]
    const N: usize = 32;

    let bump = AtomicBump::new(static_buf!([u8; 256]));
    let guard = Barrier::new(3);

    thread::scope(|s| {
        s.spawn(|| {
            for _ in 0..N {
                guard.wait();
                let _1 = Box::try_new_in(0_i16, &bump).unwrap();
                let _2 = Box::try_new_in(0_i16, &bump).unwrap();
                let _3 = Box::try_new_in(0_i16, &bump).unwrap();
                let _4 = Box::try_new_in(0_i16, &bump).unwrap();
            }
        });
        s.spawn(|| {
            for _ in 0..N {
                guard.wait();
                let _1 = Box::try_new_in(0_i32, &bump).unwrap();
                let _2 = Box::try_new_in(0_i32, &bump).unwrap();
                let _3 = Box::try_new_in(0_i32, &bump).unwrap();
                let _4 = Box::try_new_in(0_i32, &bump).unwrap();
            }
        });
        s.spawn(|| {
            for _ in 0..N {
                guard.wait();
                let _1 = Box::try_new_in(0_i64, &bump).unwrap();
                let _2 = Box::try_new_in(0_i64, &bump).unwrap();
                let _3 = Box::try_new_in(0_i64, &bump).unwrap();
                let _4 = Box::try_new_in(0_i64, &bump).unwrap();
            }
        });
    })
}
