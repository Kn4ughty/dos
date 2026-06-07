use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};

static LOGGER: Logger = Logger;

/// # Errors
/// Errors if called multiple times
pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Info))
}

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            if record.metadata().level() <= Level::Warn {
                crate::vga_println!("{} - {}", record.level(), record.args());
            }
            crate::serial_println!("{} - {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}
