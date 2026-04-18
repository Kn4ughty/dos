use lazy_static::lazy_static;
use spin::Mutex;

use crate::port::Port;

/// See https://wiki.osdev.org/Serial_Ports#Programming_the_Serial_Communications_Port for more info

pub struct Writer {
    p_read_write: Port<u8>,
    p_interrupt: Port<u8>,
    p_fifo_control: Port<u8>,
    p_line_control: Port<u8>,
    p_modem_control: Port<u8>,
    p_line_status: Port<u8>,
}

impl Writer {
    /// #Safety.
    /// Caller mist ensure the port range os valid for a serial UART port.
    pub unsafe fn new(port: u16) -> Result<Writer, ()> {
        let mut writer = Writer {
            p_read_write: Port::new(port + 0),
            p_interrupt: Port::new(port + 1),
            p_fifo_control: Port::new(port + 2),
            p_line_control: Port::new(port + 3),
            p_modem_control: Port::new(port + 4),
            p_line_status: Port::new(port + 5),
        };

        writer.init_serial()?;

        Ok(writer)
    }

    fn write_port(port: &mut Port<u8>, val: u8) {
        // Safety
        // Ports should have been verified safe by caller
        unsafe { port.write(val) }
    }

    fn read_port(port: &mut Port<u8>) -> u8 {
        // Safety
        // Ports should have been verified safe by caller
        unsafe { port.read() }
    }

    fn init_serial(&mut self) -> Result<(), ()> {
        Self::write_port(&mut self.p_interrupt, 0x00); // Disable all interrupts
        Self::write_port(&mut self.p_line_control, 0x80); // Enable DLAB (set baud rate divisor)
        Self::write_port(&mut self.p_read_write, 0x03); // Set divisor to 3 (lo byte) 38400 baud
        Self::write_port(&mut self.p_interrupt, 0x00); //                  (hi byte)
        Self::write_port(&mut self.p_line_control, 0x03); // 8 bits, no parity, one stop bit
        Self::write_port(&mut self.p_fifo_control, 0xC7); // Enable FIFO, clear them, with 14-byte threshold
        Self::write_port(&mut self.p_modem_control, 0x0B); // IRQs enabled, RTS/DSR set
        Self::write_port(&mut self.p_modem_control, 0x1E); // Set in loopback mode, test the serial chip

        // Check if serial is faulty (i.e: not same byte as sent)
        Self::write_port(&mut self.p_read_write, 0xAE);
        if Self::read_port(&mut self.p_read_write) != 0xAE {
            return Err(());
        }

        // If serial is not faulty set it in normal operation mode
        // (not-loopback with IRQs enabled and OUT#1 and OUT#2 bits enabled)
        Self::write_port(&mut self.p_modem_control, 0x0f);
        return Ok(());
    }

    #[allow(unused)]
    fn serial_received(&mut self) -> bool {
        return (Self::read_port(&mut self.p_line_status) & 1) == 1;
    }

    #[allow(unused)]
    fn read_serial(&mut self) -> char {
        while !self.serial_received() {
            core::hint::spin_loop();
        }

        return Self::read_port(&mut self.p_read_write) as char; // TODO, support unicode input
    }

    fn is_transmit_empty(&mut self) -> bool {
        return (Self::read_port(&mut self.p_line_status) & 0x20) != 0;
    }

    #[inline]
    pub fn write_char(&mut self, a: char) {
        while !self.is_transmit_empty() {
            core::hint::spin_loop();
        }

        // Allows outputting unicode
        let mut char_buf: [u8; 4] = [0; 4];
        a.encode_utf8(&mut char_buf);
        for i in 0..a.len_utf8() {
            Self::write_port(&mut self.p_read_write, char_buf[i]);
        }
    }

    pub fn write_string(&mut self, s: &str) {
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
