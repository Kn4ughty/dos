use crate::port::{Port, PortReadOnly};

/// Programmable Interval Timer
/// See <https://wiki.osdev.org/Programmable_Interval_Timer> for more
pub struct Pit {
    channel0: Port<u8>,
    channel1: Port<u8>,
    channel2: Port<u8>,
    command: PortReadOnly<u8>,
}
