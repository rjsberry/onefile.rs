use std::sync::{Arc, Mutex};
use std::thread;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use qcell::DoubleBufferedCell;

pub fn bench_qcell(c: &mut Criterion) {
    c.bench_function("qcell", |b| {
        b.iter(|| {
            let cell = black_box(DoubleBufferedCell::new(0));
            thread::scope(|s| {
                s.spawn(|| {
                    for i in 0..=1024 {
                        unsafe {
                            cell.write_uncontended(&i);
                        }
                    }
                });
                s.spawn(|| while black_box(cell.read()) != 1024 {});
            });
        });
    });
}

pub fn bench_std_mutex(c: &mut Criterion) {
    c.bench_function("mutex (std)", |b| {
        b.iter(|| {
            let mtx = black_box(Arc::new(Mutex::new(0)));
            thread::scope(|s| {
                s.spawn(|| {
                    for i in 0..=1024 {
                        *mtx.lock().unwrap() = i;
                    }
                });
                s.spawn(|| while black_box(*mtx.lock().unwrap()) != 1024 {});
            });
        });
    });
}

pub fn bench_parking_lot_mutex(c: &mut Criterion) {
    c.bench_function("mutex (parking_lot)", |b| {
        b.iter(|| {
            let mtx = black_box(Arc::new(parking_lot::Mutex::new(0)));
            thread::scope(|s| {
                s.spawn(|| {
                    for i in 0..=1024 {
                        *mtx.lock() = i;
                    }
                });
                s.spawn(|| while black_box(*mtx.lock()) != 1024 {});
            });
        });
    });
}

pub fn bench_flume(c: &mut Criterion) {
    c.bench_function("flume", |b| {
        b.iter(|| {
            let (tx, rx) = black_box(flume::bounded(1));
            thread::scope(|s| {
                s.spawn(|| {
                    for i in 0..=1024 {
                        let _ = tx.send(i);
                    }
                });
                s.spawn(|| loop {
                    if let Ok(1024) = black_box(rx.recv()) {
                        break;
                    }
                });
            });
        });
    });
}

criterion_group!(
    benches,
    bench_qcell,
    bench_flume,
    bench_std_mutex,
    bench_parking_lot_mutex
);
criterion_main!(benches);
