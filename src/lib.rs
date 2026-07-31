#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
// TODO update rust version so featrure flag isnt needed
#![feature(string_from_utf8_lossy_owned)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)]
#![feature(type_alias_impl_trait)]
#![feature(str_as_str)]
// Lints
#![warn(clippy::pedantic)]
#![allow(clippy::ptr_as_ptr)]
#![allow(clippy::used_underscore_items)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::new_without_default)]
#![allow(clippy::match_wildcard_for_single_variants)]
#![deny(
    clippy::alloc_instead_of_core,
    clippy::allow_attributes,
    clippy::as_pointer_underscore,
    clippy::assertions_on_result_states,
    clippy::clone_on_ref_ptr,
    clippy::decimal_literal_representation,
    clippy::default_union_representation,
    clippy::else_if_without_else,
    clippy::inline_asm_x86_att_syntax,
    clippy::precedence_bits,

    // These are good but lots of work
    // clippy::undocumented_unsafe_blocks
    // clippy::multiple_unsafe_ops_per_block

)]
#![warn(clippy::missing_assert_message)]

extern crate alloc;

use bootloader::BootInfo;
use core::panic::PanicInfo;

use log::error;

pub mod acpi;
pub mod gdt;
pub mod interrupts;
pub mod logging;
pub mod mem;
pub mod multiboot;
pub mod net;
pub mod pci;
pub mod pic;
pub mod port;
pub mod serial;
pub mod sync;
pub mod task;
pub mod time;
pub mod tryfrom;
pub mod vga_buffer;
pub mod volatile;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::vga_print!("\n");
        $crate::serial_print!("\n")
    };
    ($($arg:tt)*) => {{
        $crate::vga_print!("{}\n", format_args!($($arg)*));
        $crate::serial_print!("{}\n", format_args!($($arg)*));
    }};
}

pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) -> ! {
    use crate::port::PortWriteOnly;

    let mut qemu_port = PortWriteOnly::<u32>::new(0xf4);

    // Safety.
    // Qemu port is correct so it is okay
    unsafe { qemu_port.write(exit_code as u32) }

    // Problem with using unreachable!() here is that the panic handler could call exit_qemu, leading
    // to an infinite loop of unreachable!()
    // This location _should_ be unreachable.
    #[expect(clippy::empty_loop)]
    loop {}
}

pub fn hlt_loop() -> ! {
    x86_64::instructions::interrupts::without_interrupts(|| {
        loop {
            // Safe since it cannot possibly compromise memory safety
            x86_64::instructions::hlt();
        }
    })
}

pub fn init() {
    logging::init().expect("logger init called once");
    gdt::init();
    time::init();
    interrupts::init_idt();
    unsafe { interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
}

pub fn memory_init(bootinfo: &'static BootInfo) {
    use x86_64::VirtAddr;
    log::debug!("initialising memory");

    let phys_mem_offset = VirtAddr::new(bootinfo.physical_memory_offset);
    log::trace!("Phys mem offset: {:?}", phys_mem_offset);

    let mut mapper = unsafe { mem::init(phys_mem_offset) };

    let mut frame_allocator = unsafe { mem::BootInfoFrameAllocator::init(&bootinfo.memory_map) };

    mem::allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("Heap initialzation failed");

    log::debug!("Memory init done");
}

pub fn post_memory_init() {
    log::debug!("Post memory init");
    net::init();
    log::debug!("Post memory init");
    acpi::init();
}

#[cfg(test)]
use bootloader::entry_point;

#[cfg(test)]
entry_point!(test_kernel_main);

/// Entry point for `cargo xtest`
#[cfg(test)]
fn test_kernel_main(_boot_info: &'static BootInfo) -> ! {
    init();
    test_main();
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}

pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }

    exit_qemu(QemuExitCode::Success);
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    error!("[failed]\n");
    error!("{:#?}\n", info);
    exit_qemu(QemuExitCode::Failed);
}
