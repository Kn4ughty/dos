#![no_std]
#![no_main]

mod vga_buffer;
mod volatile;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{:#?}", info);
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    println!("Hello world!");

    #[allow(clippy::empty_loop)]
    loop {}
}
