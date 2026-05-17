#![no_std]
#![no_main]
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

    use alloc::{rc::Rc, vec};

    let reference_counted = Rc::new(vec![1, 2, 3]);
    let cloned_reference = reference_counted.clone();
    println!(
        "current reference count is {}",
        Rc::strong_count(&cloned_reference)
    );
    core::mem::drop(reference_counted);
    println!(
        "reference count is {} now",
        Rc::strong_count(&cloned_reference)
    );

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
