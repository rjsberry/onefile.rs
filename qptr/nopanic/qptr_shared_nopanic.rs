#![no_std]
#![no_main]

extern crate libc;

use core::any::Any;
use core::panic::PanicInfo;

use qptr::{make_static_shared, Shared};

#[no_mangle]
unsafe extern "C" fn main(argc: isize, _argv: *const *const u8) -> isize {
    let ptr: Shared<dyn Any> = make_static_shared!(|| -> isize { argc - 1 }).unwrap_unchecked();
    *ptr.downcast::<isize>().unwrap_unchecked()
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    extern "Rust" {
        fn __undefined_symbol() -> !;
    }
    unsafe { __undefined_symbol() }
}
