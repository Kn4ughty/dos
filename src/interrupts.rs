use x86_64::instructions::interrupts;
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

/// Contains the the irq handler functions.
/// This is needed so that handlers can be registed at runtime, as PCI devices can create
/// interrupts, and handling them is good.
pub static DYN_INTERRUPTS: Mutex<[fn(); 16]> = Mutex::new([default_interrupt_handler; 16]);

/// Used for the numbered irq handlers
fn default_interrupt_handler() {}

fn interrupt_index(irq: u8) -> u8 {
    PIC_1_OFFSET + irq
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum InterruptIndex {
    Timer = 0,
    Keyboard,
}

impl InterruptIndex {
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
        idt[interrupt_index(0)].set_handler_fn(irq0_handler);
        idt[interrupt_index(1)].set_handler_fn(irq1_handler);
        idt[interrupt_index(2)].set_handler_fn(irq2_handler);
        idt[interrupt_index(3)].set_handler_fn(irq3_handler);
        idt[interrupt_index(4)].set_handler_fn(irq4_handler);
        idt[interrupt_index(5)].set_handler_fn(irq5_handler);
        idt[interrupt_index(6)].set_handler_fn(irq6_handler);
        idt[interrupt_index(7)].set_handler_fn(irq7_handler);
        idt[interrupt_index(8)].set_handler_fn(irq8_handler);
        idt[interrupt_index(9)].set_handler_fn(irq9_handler);
        idt[interrupt_index(10)].set_handler_fn(irq10_handler);
        idt[interrupt_index(11)].set_handler_fn(irq11_handler);
        idt[interrupt_index(12)].set_handler_fn(irq12_handler);
        idt[interrupt_index(13)].set_handler_fn(irq13_handler);
        idt[interrupt_index(14)].set_handler_fn(irq14_handler);
        idt[interrupt_index(15)].set_handler_fn(irq15_handler);

        set_irq_handler(InterruptIndex::Timer.as_u8(), timer_interrupt_handler);
        set_irq_handler(InterruptIndex::Keyboard.as_u8(), keyboard_interrupt_handler);
        idt
    };
}

macro_rules! irq_handler {
    ($handler:ident, $irq:expr) => {
        pub extern "x86-interrupt" fn $handler(_: InterruptStackFrame) {
            //println!("interrupt: {}", $irq);
            let handler = DYN_INTERRUPTS.lock()[$irq];
            handler();
            unsafe { PICS.lock().notify_end_of_interrupt(interrupt_index($irq)) }
        }
    };
}
irq_handler!(irq0_handler, 0);
irq_handler!(irq1_handler, 1);
irq_handler!(irq2_handler, 2);
irq_handler!(irq3_handler, 3);
irq_handler!(irq4_handler, 4);
irq_handler!(irq5_handler, 5);
irq_handler!(irq6_handler, 6);
irq_handler!(irq7_handler, 7);
irq_handler!(irq8_handler, 8);
irq_handler!(irq9_handler, 9);
irq_handler!(irq10_handler, 10);
irq_handler!(irq11_handler, 11);
irq_handler!(irq12_handler, 12);
irq_handler!(irq13_handler, 13);
irq_handler!(irq14_handler, 14);
irq_handler!(irq15_handler, 15);

pub fn set_irq_handler(irq: u8, handler: fn()) {
    interrupts::without_interrupts(|| {
        let mut handlers = DYN_INTERRUPTS.lock();
        handlers[irq as usize] = handler;
        // mask?
    });
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

fn timer_interrupt_handler() {
    //print!(".");
}

fn keyboard_interrupt_handler() {
    use crate::port::Port;

    let mut port = Port::new(0x60); // Port of PS/2 controller
    let scancode: u8 = unsafe { port.read() };
    crate::task::keyboard::add_scancode(scancode);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{stack_frame:#?}\nError_code={error_code:#x}")
}
