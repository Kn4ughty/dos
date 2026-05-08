#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)]
#![feature(type_alias_impl_trait)]

// extern crate alloc;

use core::panic::PanicInfo;

use crate::multiboot::BootInformationFormat;
//
// // pub mod allocator;
pub mod gdt;
pub mod interrupts;
pub mod memory;
pub mod pic;
pub mod port;
pub mod serial;
pub mod vga_buffer;
pub mod volatile;

pub mod multiboot;

// #[unsafe(no_mangle)]
// pub extern "C" fn _start() -> ! {
//     kernel_main();
// }

// fn kernel_main() -> ! {
// init();
// vga_println!("Hello world!");
//
// vga_println!("{:?}", boot_info);
//
// let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
// let mut mapper = unsafe { memory::init(phys_mem_offset) };
// let mut frame_allocator =
//     unsafe { memory::BootInfoFrameAllocator::init(&boot_info.memory_map) };
//
// allocator::init_heap(&mut mapper, &mut frame_allocator).expect("Heap initialzation failed");
//
// let reference_counted = Rc::new(vec![1, 2, 3]);
// let cloned_reference = reference_counted.clone();
// vga_println!(
//     "current reference count is {}",
//     Rc::strong_count(&cloned_reference)
// );
// core::mem::drop(reference_counted);
// vga_println!(
//     "reference count is {} now",
//     Rc::strong_count(&cloned_reference)
// );
//
// #[cfg(test)]
// test_main();
//
//     vga_println!("Did not crash. End of main");
//     hlt_loop();
// }

#[unsafe(no_mangle)]
pub extern "C" fn rust_main(multiboot_information_address: usize) -> ! {
    vga_println!("Hello from rustland!");
    init();

    let bif = unsafe { BootInformationFormat::load(multiboot_information_address) };

    let mmap = bif
        .get::<multiboot::MemoryMap>()
        .expect("can get memory_map");

    let mentry = mmap.get_all_entries();
    for entry in mentry {
        println!("{:#?}", entry);
    }

    let elf = bif.get::<multiboot::ELFSymbols>().expect("get elf");
    println!("{:#?}", elf);

    // let bs = b"hello from rustland!";
    // let color = 0x
    //
    // let buffer_ptr = (0xb8000 + 1988) as *mut _;
    // unsafe { *buffer_ptr = [0x1f67] };

    vga_println!("Finished");
    hlt_loop();
}

// #[unsafe(no_mangle)]
// pub extern "C" fn kernel_main() {}
//
// #[lang = "eh_personality"]
// #[unsafe(no_mangle)]
// pub extern "C" fn eh_personality() {}

// #[lang = "panic_fmt"]
// #[no_mangle]
// pub extern "C" fn panic_fmt() -> ! {
//     loop {}
// }

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
    loop {}
}
//
pub fn hlt_loop() -> ! {
    use core::arch::asm;
    loop {
        // Safe since it cannot possibly compromise memory safety
        unsafe { asm!("hlt") }
    }
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

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{:#?}", info);
    hlt_loop();
}

pub fn init() {
    gdt::init();
    // interrupts::init_idt();
    // unsafe { interrupts::PICS.lock().initialize() };
    // x86_64::instructions::interrupts::enable();
}
//
// #[cfg(test)]
// bootloader::entry_point!(test_kernel_main);
//
// #[cfg(test)]
// fn test_kernel_main(_boot_info: &'static bootloader::BootInfo) -> ! {
//     init();
//     test_main();
//
//     hlt_loop();
// }
