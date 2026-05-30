#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)]
#![feature(type_alias_impl_trait)]
// #![feature(trait_alias)]

extern crate alloc;

use core::panic::PanicInfo;

pub mod memory;

pub mod allocator;
pub mod gdt;
pub mod interrupts;
pub mod multiboot;
pub mod pci;
pub mod pic;
pub mod port;
pub mod serial;
pub mod spinlock;
pub mod task;
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
    ($($arg:tt)*) => {
        {
            $crate::vga_print!("{}\n", format_args!($($arg)*));
            $crate::serial_print!("{}\n", format_args!($($arg)*));
        }
    };
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
    #[allow(clippy::empty_loop)]
    loop {}
}

pub fn hlt_loop() -> ! {
    use core::arch::asm;
    loop {
        // Safe since it cannot possibly compromise memory safety
        unsafe { asm!("hlt") }
    }
}

pub fn init() {
    gdt::init();
    interrupts::init_idt();
    unsafe { interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
}

#[cfg(test)]
use bootloader::{BootInfo, entry_point};

#[cfg(test)]
entry_point!(test_kernel_main);

/// Entry point for `cargo xtest`
#[cfg(test)]
fn test_kernel_main(_boot_info: &'static BootInfo) -> ! {
    init();
    test_main();
    hlt_loop();
}

// #[cfg(not(test))]
// #[panic_handler]
// fn panic(info: &PanicInfo) -> ! {
//     println!("{}", info);
//     hlt_loop();
// }

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
    serial_println!("[failed]\n");
    serial_println!("{:#?}\n", info);
    exit_qemu(QemuExitCode::Failed);
}
