#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{BootInfo, entry_point};

use os::{
    init,
    mem::{self, allocator},
    pci::{self, rtl8139::RTL8139},
    println, vga_println,
};

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

    find_rtl();

    // rtl.init();
    // println!("{}", rtl.mac_string());
    //
    // rtl.send_arp();
    // for _ in 0..1_000_000 {
    //     core::hint::spin_loop();
    // }
    // unsafe {
    //     let isr = rtl.ports.interrupt_status.read(); // need to make ports pub temporarily
    //     println!("ISR after send: {:#06x}", isr);
    // }

    #[cfg(test)]
    test_main();

    use os::task::{Task, executor::Executor, keyboard, network};

    let mut executor = Executor::new();
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.spawn(Task::new(network::get_packet()));
    executor.run();
}

fn find_rtl() {
    for bus in 0..=255 {
        for device in 0..=31 {
            let mut pci_device = pci::PCIDevice::new(bus, device);
            if let Some(header) = pci_device.get_header() {
                // println!("{:#?}", header);
                // println!(" {:#0x}", header.base_addr0);
                if header.vendor_id == pci::rtl8139::VENDOR_ID {
                    println!("Found rtl");
                    println!("{:#?}", header);
                    pci_device.enable_bus_mastering();

                    let mut rtl = pci::rtl8139::RTL8139::new((header.base_addr0 & 0xFFFC) as u16);
                    rtl.init();
                    rtl.send_arp();
                    rtl.register_interrupts(header.interrupt_line);
                }
            }
        }
    }
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
