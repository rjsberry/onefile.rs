#![no_std]
#![no_main]

extern crate libc;

use core::panic::PanicInfo;

use qcell::DoubleBufferedCell;

static CELL: DoubleBufferedCell<usize> = DoubleBufferedCell::new(0);

#[no_mangle]
unsafe extern "C" fn main(_argc: isize, _argv: *const *const u8) -> isize {
    CELL.write_uncontended(&1);
    if CELL.read() == 1 {
        0
    } else {
        1
    }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    extern "Rust" {
        fn __undefined_symbol() -> !;
    }
    unsafe { __undefined_symbol() }
}
