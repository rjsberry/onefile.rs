#![no_std]
#![no_main]

extern crate libc;

use core::panic::PanicInfo;
use core::slice;
use core::str;

#[no_mangle]
extern "C" fn main(argc: isize, argv: *const *const u8) -> isize {
    if argc == 2 {
        let jsonb = unsafe {
            let jsonptr = *argv.offset(1);
            slice::from_raw_parts(jsonptr, libc::strlen(jsonptr as *const libc::c_char))
        };

        match str::from_utf8(jsonb) {
            Ok(json) if qjson::validate::<32>(json).is_ok() => return 0,
            _ => (),
        }
    }

    1
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    extern "Rust" {
        fn __undefined_symbol() -> !;
    }
    unsafe { __undefined_symbol() }
}
