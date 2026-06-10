use core::sync::atomic::AtomicUsize;

use crate::vga_buffer::{_print_coloured, Colour};

use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};

const STARTUP_LEVEL: Level = Level::Debug;

static LOGGER: Logger = Logger;

static LOG_LEVEL: AtomicUsize = AtomicUsize::new(STARTUP_LEVEL as usize);

/// # Errors
/// Errors if called multiple times
// If logging is slow, decrease maxmium log level
pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Trace))
}

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() as usize <= LOG_LEVEL.load(core::sync::atomic::Ordering::Acquire)
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
