use core::fmt::Display;

use lazy_static::lazy_static;

use crate::{port::Port, spinlock::Mutex, tryfrom::tryfrom};

lazy_static! {
    pub static ref CMOS: Mutex<Cmos> = Mutex::new(Cmos::new());
}

/// <https://wiki.osdev.org/CMOS>
#[non_exhaustive]
pub struct Cmos {
    register_number: Port<u8>,
    config: Port<u8>,
}

impl Cmos {
    /// Users of this must access via the mutex. (since ports are global)
    #[must_use]
    fn new() -> Self {
        let mut cmos = Cmos {
            register_number: Port::new(0x70),
            config: Port::new(0x71),
        };
        const STATB: u8 = 0x0B;

        let statb = cmos.read_reg(STATB);
        // Enable binary mode and 24 hour mode
        cmos.write_reg(STATB, statb | 4 | 2);

        cmos
    }

    fn read_reg(&mut self, reg: u8) -> u8 {
        unsafe {
            self.register_number.write(reg);
            // May need to wait for update on real hardware
            self.config.read()
        }
    }

    fn write_reg(&mut self, reg: u8, val: u8) {
        unsafe {
            self.register_number.write(reg);
            // May need to wait for update on real hardware
            self.config.write(val);
        }
    }

    fn get_year(&mut self) -> u16 {
        let year = self.read_reg(0x09);

        let century_enabled = crate::acpi::FADT
            .get()
            .expect("acpi must initialised")
            .century
            != 0;

        let century = if century_enabled {
            self.read_reg(0x32)
        } else {
            20
        };

        u16::from(century) * 100 + u16::from(year)
    }

    pub fn get_datetime(&mut self) -> DateTime {
        DateTime {
            seconds: self.read_reg(0x00),
            minutes: self.read_reg(0x02),
            hours: self.read_reg(0x04),
            weekday: Weekday::try_from(self.read_reg(0x06)).expect("faulty CMOS"),
            day_of_month: self.read_reg(0x07),
            month: Month::try_from(self.read_reg(0x08)).expect("faulty CMOS"),
            year: self.get_year(),
        }
    }
}

/// Unvalidated
pub struct DateTime {
    seconds: u8,
    minutes: u8,
    hours: u8,
    weekday: Weekday,
    day_of_month: u8,
    month: Month,
    /// contains best guess.
    /// Also screw people living 63,510 years in the future
    year: u16,
}

impl Display for DateTime {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Example
        // 21:40:14 Monday, 8 Jun, 26
        f.write_fmt(format_args!(
            "{}:{:02}:{:02} {:?}, {} {:?}, {}",
            self.hours,
            self.minutes,
            self.seconds,
            self.weekday,
            self.day_of_month,
            self.month,
            self.year
        ))
    }
}

tryfrom! {
    #[repr(u8)]
    #[derive(Debug)]
    pub enum Weekday {
        Sunday = 1,
        Monday = 2,
        Tuesday = 3,
        Wednesday = 4,
        Thursday = 5,
        Friday = 6,
        Saturday = 7,
    }, u8
}

tryfrom! {
    #[repr(u8)]
    #[derive(Debug)]
    pub enum Month {
        Jan = 1,
        Feb = 2,
        Mar = 3,
        Apr = 4,
        May = 5,
        Jun = 6,
        Jul = 7,
        Aug = 8,
        Sep = 9,
        Oct = 10,
        Nov = 11,
        Dec = 12,
    }, u8
}
