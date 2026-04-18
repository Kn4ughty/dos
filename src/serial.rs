use crate::println;
use core::arch::asm;

use lazy_static::lazy_static;
use spin::Mutex;

// TODO. Work out safety requirements

pub fn outb(port: u16, val: u8) {
    unsafe {
        asm!("out dx, al", in("al") val, in("dx") port);
    }
}

pub fn inb(port: u16) -> u8 {
    let mut ret: u8 = 0;
    unsafe {
        asm!("in al, dx", out("al") ret, in("dx") port);
    }
    return ret;
}

pub struct Writer {
    port: u16,
}

impl Writer {
    //! Safety.
    //! Port must be valid and safe for life time of the device.
    pub unsafe fn new(port: u16) -> Result<Writer, ()> {
        let writer = Writer { port };

        writer.init_serial()?;

        Ok(writer)
    }

    fn init_serial(&self) -> Result<(), ()> {
        outb(self.port + 1, 0x00); // Disable all interrupts
        outb(self.port + 3, 0x80); // Enable DLAB (set baud rate divisor)
        outb(self.port + 0, 0x03); // Set divisor to 3 (lo byte) 38400 baud
        outb(self.port + 1, 0x00); //                  (hi byte)
        outb(self.port + 3, 0x03); // 8 bits, no parity, one stop bit
        outb(self.port + 2, 0xC7); // Enable FIFO, clear them, with 14-byte threshold
        outb(self.port + 4, 0x0B); // IRQs enabled, RTS/DSR set
        outb(self.port + 4, 0x1E); // Set in loopback mode, test the serial chip
        outb(self.port + 0, 0xAE); // Test serial chip (send byte 0xAE and check if
        // serial returns same byte)

        // Check if serial is faulty (i.e: not same byte as sent)
        if inb(self.port + 0) != 0xAE {
            return Err(());
        }

        // If serial is not faulty set it in normal operation mode
        // (not-loopback with IRQs enabled and OUT#1 and OUT#2 bits enabled)
        outb(self.port + 4, 0x0F);
        return Ok(());
    }

    #[allow(unused)]
    fn serial_received(&self) -> bool {
        return (inb(self.port + 5) & 1) == 1;
    }

    #[allow(unused)]
    fn read_serial(&self) -> char {
        while !self.serial_received() {
            core::hint::spin_loop();
        }

        return inb(self.port) as char; // TODO, support unicode input
    }

    fn is_transmit_empty(&self) -> bool {
        return (inb(self.port + 5) & 0x20) != 0;
    }

    #[inline]
    pub fn write_char(&self, a: char) {
        while !self.is_transmit_empty() {
            core::hint::spin_loop();
        }

        // Allows outputting unicode
        let mut char_buf: [u8; 4] = [0; 4];
        a.encode_utf8(&mut char_buf);
        for i in 0..a.len_utf8() {
            outb(self.port, char_buf[i]);
        }
    }

    pub fn write_string(&self, s: &str) {
        // let _ = s.chars().map(|c| self.write_char(c));
        for c in s.chars() {
            self.write_char(c)
        }
    }
}

use core::fmt::Write;
impl Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

lazy_static! {
    pub static ref SERIAL1: Mutex<Writer> =
        Mutex::new(unsafe { Writer::new(0x3F8).expect("can create serial writer") });
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => ($crate::serial::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($($arg:tt)*) => ($crate::serial_print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    // Disable interrupts to prevent deadlocking
    x86_64::instructions::interrupts::without_interrupts(|| {
        SERIAL1.lock().write_fmt(args).unwrap();
    });
}
