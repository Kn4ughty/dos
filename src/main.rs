#![no_std]
#![no_main]
#![feature(box_as_ptr)]
#![feature(custom_test_frameworks)]
#![test_runner(os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{BootInfo, entry_point};

use os::{allocator, hlt_loop, init, memory, println, vga_println};

// // 1. THE BOOTIMAGE ENTRY POINT (For your current testing scaffolding)

#[cfg(feature = "bootimage")]
entry_point!(bootimage_start);

#[cfg(feature = "bootimage")]
fn bootimage_start(boot_info: &'static BootInfo) -> ! {
    k_main(boot_info);
}

#[cfg(not(feature = "bootimage"))]
#[no_mangle]
pub extern "C" fn multiboot_start(multiboot_information_address: usize, phys_mem_offset: u64) -> ! {
    let bif = unsafe { BootInformationFormat::load(multiboot_information_address) };

    let elf = bif.get::<multiboot::ELFSymbols>().expect("get elf");
    println!("{:#?}", elf);

    let k_start = elf.get_sections().map(|s| s.start_addr()).min().unwrap() as usize;
    let k_end = elf.get_sections().map(|s| s.end_addr()).max().unwrap() as usize;
    println!("kernl start: {:#x}, end: {:#x}", k_start, k_end);

    let m_start = bif.start_addr() as usize;
    let m_end = bif.end_addr() as usize;
    println!("multi start: {:#x}, end: {:#x}", m_start, m_end);
    k_main();
}

fn k_main(bootinfo: &'static BootInfo) -> ! {
    // fn k_main(boot_info: &'static BootInfo) -> ! {
    vga_println!("Hello from rustland!");
    init();

    use x86_64::VirtAddr;

    let phys_mem_offset = VirtAddr::new(bootinfo.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };

    let mut frame_allocator = unsafe { memory::BootInfoFrameAllocator::init(&bootinfo.memory_map) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("Heap initialzation failed");

    // This seems to be a PCI controller.
    // https://theretroweb.com/chips/2755
    for bus in 0..=255 {
        for device in 0..=31 {
            let mut pci_device = os::pci::PCIDevice::new(bus, device);
            if let Some(header) = pci_device.get_header() {
                println!("{:#?}", header);
                println!(" {:#0x}", header.base_addr0);
            }
        }
    }

    #[cfg(test)]
    test_main();

    vga_println!("Finished");
    hlt_loop();
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
