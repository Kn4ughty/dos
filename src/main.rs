#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{BootInfo, entry_point};

use os::{init, mem, mem::allocator, pci, println, vga_println};

// // 1. THE BOOTIMAGE ENTRY POINT (For your current testing scaffolding)

#[cfg(feature = "bootimage")]
entry_point!(bootimage_start);

#[cfg(feature = "bootimage")]
fn bootimage_start(boot_info: &'static BootInfo) -> ! {
    k_main(boot_info);
}

fn k_main(bootinfo: &'static BootInfo) -> ! {
    // fn k_main(boot_info: &'static BootInfo) -> ! {
    vga_println!("Hello from rustland!");
    init();

    use x86_64::VirtAddr;

    let phys_mem_offset = VirtAddr::new(bootinfo.physical_memory_offset);
    let mut mapper = unsafe { mem::init(phys_mem_offset) };

    let mut frame_allocator = unsafe { mem::BootInfoFrameAllocator::init(&bootinfo.memory_map) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("Heap initialzation failed");

    for bus in 0..=255 {
        for device in 0..=31 {
            let mut pci_device = pci::PCIDevice::new(bus, device);
            if let Some(header) = pci_device.get_header() {
                // println!("{:#?}", header);
                // println!(" {:#0x}", header.base_addr0);
                if header.vendor_id == pci::rtl8139::VENDOR_ID {
                    println!("Found rtl");
                    let mut rtl = pci::rtl8139::RTL8139::new(header.base_addr0 as u16);
                    rtl.init();
                    println!("{}", rtl.mac_string());
                }
            }
        }
    }

    #[cfg(test)]
    test_main();

    use os::task::{Task, executor::Executor, keyboard};

    let mut executor = Executor::new();
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run();
}

use core::panic::PanicInfo;

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    os::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    os::test_panic_handler(info)
}
