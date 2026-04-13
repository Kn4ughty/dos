#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
#[repr(u8)]
pub enum Colour {
    Black = 0,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Colour, background: Colour) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)] // Guarantees the ordering of fields
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

#[repr(transparent)]
struct Buffer {
    chars: [[ScreenChar; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
    column_position: usize,
    row_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl Writer {
    fn write_byte(&mut self, byte: u8) {
        if byte == b'\n' {
            self.new_line();
            return;
        }
        if self.column_position >= BUFFER_WIDTH {
            self.new_line();
        }

        // let row = BUFFER_HEIGHT - 1;
        // let col =
        self.buffer.chars[self.row_position][self.column_position] = ScreenChar {
            ascii_character: byte,
            color_code: self.color_code,
        };
        self.column_position += 1;
    }

    fn new_line(&mut self) {
        /* todo */
        self.row_position += 1;
        self.column_position = 0;
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            if matches!(byte, 0x20..=0x7e | b'\n') {
                self.write_byte(byte);
            } else {
                self.write_byte(0xfe);
            }
        }
    }
}

pub fn print_test() {
    let mut writer = Writer {
        column_position: 0,
        row_position: 0,
        color_code: ColorCode::new(Colour::White, Colour::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    };
    writer.write_string("Hello Wo😀rld!");
}
