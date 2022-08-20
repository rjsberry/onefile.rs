use std::thread;

use qcell::DoubleBufferedCell;

const ITER: usize = 1024 * 1024;

fn main() {
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
