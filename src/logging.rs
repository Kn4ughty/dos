use crate::vga_buffer::{_print_coloured, Colour};

use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};

static LOGGER: Logger = Logger;

/// # Errors
/// Errors if called multiple times
pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Debug))
}

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            if record.metadata().level() <= Level::Info {
                _print_coloured(
                    format_args!("{}", record.level()),
                    level_to_colour(record.level()),
                    Colour::Black,
                );
                crate::vga_println!(" - {}", record.args());
            }

            crate::serial_println!("{} - {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

fn level_to_colour(level: Level) -> Colour {
    match level {
        Level::Error => Colour::Red,
        Level::Warn => Colour::LightRed,
        Level::Info => Colour::LightCyan,
        Level::Debug => Colour::Cyan,
        Level::Trace => Colour::LightBlue,
    }
}
