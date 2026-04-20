use lazy_static::lazy_static;
use spin::Mutex;

use crate::port::Port;
use crate::volatile::Volatile;

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
#[repr(u8)]
pub enum Colour {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
/// Contains foreground and background colours
struct ColorCode(u8);

impl ColorCode {
    const fn new(foreground: Colour, background: Colour) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}
const TEXT_COLOUR: ColorCode = ColorCode::new(Colour::White, Colour::Black);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)] // Guarantees the ordering of fields
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

// See http://www.osdever.net/FreeVGA/vga/textcur.htm for more
struct Cursor {
    address_register: Port<u8>,
    data_register: Port<u8>,
}

impl Cursor {
    fn new() -> Cursor {
        Cursor {
            address_register: Port::new(0x3D4),
            data_register: Port::new(0x3D5),
        }
    }

    #[allow(unused)]
    fn disable(&mut self) {
        // Safety! is ok
        unsafe {
            self.address_register.write(0x0A);
            self.data_register.write(0x10);
        }
    }

    /// Implicitly also re-enables the cursor
    fn set_position(&mut self, row: usize, col: usize) {
        // she'll be right
        let index: u16 = (BUFFER_WIDTH as u16) * (row as u16) + (col as u16);
        unsafe {
            self.address_register.write(0x0F);
            // Will always be okay since yk &0xFF
            self.data_register.write((index & 0xFF) as u8);

            self.address_register.write(0x0E);
            self.data_register.write(((index >> 8) & 0xFF) as u8);
        }
    }
}

pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    cursor: Cursor,
    buffer: &'static mut Buffer,
}

impl Writer {
    fn new() -> Writer {
        let mut writer = Writer {
            column_position: 0,
            color_code: TEXT_COLOUR,
            cursor: Cursor::new(),
            // Safety. This is the correct address for the vga buffer
            buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
        };
        writer.clear_screen();

        writer
    }

    fn write_byte(&mut self, byte: u8) {
        if byte == b'\n' {
            self.new_line();
            return;
        }
        if self.column_position >= BUFFER_WIDTH {
            self.new_line();
        }

        self.buffer.chars[BUFFER_HEIGHT - 1][self.column_position].write(ScreenChar {
            ascii_character: byte,
            color_code: self.color_code,
        });
        self.column_position += 1;
    }

    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(character);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: TEXT_COLOUR,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }

    fn clear_screen(&mut self) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: TEXT_COLOUR,
        };
        for col in 0..BUFFER_WIDTH {
            for row in 0..BUFFER_HEIGHT {
                self.buffer.chars[row][col].write(blank);
            }
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            // check if it is printable ascii
            if matches!(byte, b' '..=b'~' | b'\n') {
                self.write_byte(byte);
            } else {
                // Scary recursion
                let r = write!(self, "0x{:02x}", byte);
                if let Err(e) = r {
                    use crate::serial_println;
                    serial_println!("WRITE UNKNOWN CHARACTER ERROR: {:?}", e);
                }
            }
        }
        self.cursor
            .set_position(BUFFER_HEIGHT - 1, self.column_position)
    }
}

use core::fmt::Write;
impl Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

// cannot use a std::LazyLock as no std. So using lazy_static
lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer::new());
}

#[macro_export]
macro_rules! vga_print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! vga_println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    // Disable interrupts to prevent deadlocking
    x86_64::instructions::interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap();
    });
}

#[test_case]
fn test_println_simple() {
    vga_println!("test_println_simple output");
}

#[test_case]
fn test_println_many() {
    for _ in 0..200 {
        vga_println!("test_println_many output");
    }
}

#[test_case]
fn test_println_longgg() {
    vga_println!(
        "test_println_longgg output very long line of text that is sure to take up more than one line on the display, and hence test if text wrapping does not panic"
    );
}

#[test_case]
fn test_println_appear() {
    vga_println!(); // So that a print!() call cannot mess up the logic.
    let s = "FLAG";
    vga_println!("{}", s);
    // Disable interrupts to prevent other text from being printed to the screen
    x86_64::instructions::interrupts::without_interrupts(|| {
        for (i, c) in s.chars().enumerate() {
            let screen_char = WRITER.lock().buffer.chars[BUFFER_HEIGHT - 2][i].read();
            assert_eq!(char::from(screen_char.ascii_character), c);
        }
    });
}
