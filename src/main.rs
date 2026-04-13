#![no_std]
#![no_main]

mod vga_buffer;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    // let vga_buf = 0xb8000 as *mut u8;
    //
    // for (i, &byte) in HELLO.iter().enumerate() {
    //     unsafe {
    //         *vga_buf.offset(i as isize * 2) = byte;
    //         *vga_buf.offset(i as isize * 2 + 1) = 0xb; // b??
    //     }
    // }
    vga_buffer::print_test();

    #[allow(clippy::empty_loop)]
    loop {}
}
