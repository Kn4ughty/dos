#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(os::test_runner)]
#![reexport_test_harness_main = "test_main"]

#[expect(unused)]
use log::{debug, error, info, trace, warn};

extern crate alloc;

use bootloader::{BootInfo, entry_point};

use os::{
    init,
    mem::{self, allocator},
    net, println, vga_println,
};

#[cfg(feature = "bootimage")]
entry_point!(bootimage_start);

#[cfg(feature = "bootimage")]
fn bootimage_start(boot_info: &'static BootInfo) -> ! {
    k_main(boot_info);
}

fn k_main(bootinfo: &'static BootInfo) -> ! {
    vga_println!("Hello from rustland!");
    init();

    use x86_64::VirtAddr;

    let phys_mem_offset = VirtAddr::new(bootinfo.physical_memory_offset);
    let mut mapper = unsafe { mem::init(phys_mem_offset) };

    let mut frame_allocator = unsafe { mem::BootInfoFrameAllocator::init(&bootinfo.memory_map) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("Heap initialzation failed");

    net::init();

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
