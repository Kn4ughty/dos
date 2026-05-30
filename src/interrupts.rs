use core::ops::IndexMut;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use lazy_static::lazy_static;

use crate::pic::ChainedPics;
use crate::{gdt, hlt_loop};
use crate::{println, vga_println};

use crate::spinlock::Mutex;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

// Safety. The offsets need to be correct
pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    // why lol
    fn as_u8(self) -> u8 {
        self as u8
    }
}

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        // SAFETY. double_fault_ist_index is correct as we source directly from gdt
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt.index_mut(InterruptIndex::Timer.as_u8()).set_handler_fn(timer_interrupt_handler);
        idt.index_mut(InterruptIndex::Keyboard.as_u8()).set_handler_fn(keyboard_interrupt_handler);
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    vga_println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}
#[test_case]
fn test_breakpoint_exception() {
    x86_64::instructions::interrupts::int3();
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed address: {:?}", Cr2::read());
    println!("Error code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // vga_print!(".");

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use crate::port::Port;

    let mut port = Port::new(0x60); // Port of PS/2 controller
    let scancode: u8 = unsafe { port.read() };
    crate::task::keyboard::add_scancode(scancode);

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{stack_frame:#?}\nError_code={error_code:#x}")
}
