#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(os::test_runner)]
#![reexport_test_harness_main = "test_main"]

#[expect(unused)]
use log::{debug, error, info, trace, warn};

extern crate alloc;

use bootloader::{BootInfo, entry_point};

use os::{net, println, vga_println};

#[cfg(feature = "bootimage")]
entry_point!(bootimage_start);

#[cfg(feature = "bootimage")]
fn bootimage_start(boot_info: &'static BootInfo) -> ! {
    k_main(boot_info);
}

fn k_main(bootinfo: &'static BootInfo) -> ! {
    vga_println!("Hello from rustland!");
    os::init();
    os::memory_init(bootinfo);
    os::post_memory_init();

    let mut cmos = os::time::rtc::Cmos::new();
    println!("{}", cmos.get_datetime());

    #[cfg(test)]
    test_main();

    use os::task::{Task, executor::Executor, keyboard};

    let mut executor = Executor::new();
    executor.spawn(Task::new(keyboard::print_keypresses(), "Keyboard"));
    executor.spawn(Task::new(net::get_packet(), "network"));
    executor.run();
}

use core::panic::PanicInfo;

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use log::error;
    error!("{}", info);
    os::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    os::test_panic_handler(info)
}
