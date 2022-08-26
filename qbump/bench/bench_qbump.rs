#![feature(allocator_api)]

use std::alloc::{Allocator, System};
use std::mem::MaybeUninit;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use qbump::{AtomicBump, Bump};

#[inline(always)]
fn bench_allocator<A: Allocator>(alloc: &A) {
    let mut vec = black_box(Vec::<Box<[u8], _>, _>::new_in(alloc));
    vec.try_reserve(1).unwrap();

    for i in 0..1024 {
        if vec.len() == vec.capacity() {
            vec.try_reserve(2 * vec.len()).unwrap();
        }

        // "fizz-buzz" allocation for fragmentation
        match (i % 3, i % 5) {
            (0, 0) => vec.push(black_box(Box::try_new_in([0; 8], alloc)).unwrap()),
            (_, 0) => vec.push(black_box(Box::try_new_in([0; 4], alloc)).unwrap()),
            (0, _) => vec.push(black_box(Box::try_new_in([0; 2], alloc)).unwrap()),
            (_, _) => vec.push(black_box(Box::try_new_in([0; 1], alloc)).unwrap()),
        }
    }

    for _ in 0..1024 {
        drop(black_box(vec.pop()).unwrap());
    }
}

pub fn bench_system(c: &mut Criterion) {
    c.bench_function("System", |b| {
        b.iter(|| {
            bench_allocator(&System);
        });
    });
}

pub fn bench_bump(c: &mut Criterion) {
    c.bench_function("Bump", |b| {
        b.iter(|| {
            let mut buf = MaybeUninit::<[u8; 128 * 1024]>::uninit();
            let bump = unsafe { Bump::from_ptr(buf.as_mut_ptr() as *mut _, 128 * 1024) };
            bench_allocator(&bump);
        });
    });
}

pub fn bench_atomic_bump(c: &mut Criterion) {
    c.bench_function("AtomicBump", |b| {
        b.iter(|| {
            let mut buf = MaybeUninit::<[u8; 128 * 1024]>::uninit();
            let bump = unsafe { AtomicBump::from_ptr(buf.as_mut_ptr() as *mut _, 128 * 1024) };
            bench_allocator(&bump);
        });
    });
}

criterion_group!(benches, bench_system, bench_bump, bench_atomic_bump);
criterion_main!(benches);
