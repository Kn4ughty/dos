use core::f64;

/// Programmable interval timer
// https://www.scs.stanford.edu/10wi-cs140/pintos/specs/8254.pdf
use crate::port::Port;

const PIT_BASE_FREQUENCY: f64 = 1_193_182.0; // hz 

const CH0_PORT: u16 = 0x40;
const CMD_PORT: u16 = 0x43;

pub fn set_interval(hz: f64) {
    #[expect(clippy::cast_possible_truncation)]
    #[expect(clippy::cast_sign_loss, reason = "clamped")]
    let divisor = (PIT_BASE_FREQUENCY / hz).clamp(0.0, 65535.0) as u16;

    let mut data: Port<u8> = Port::new(CH0_PORT);
    let mut cmd: Port<u8> = Port::new(CMD_PORT);

    unsafe {
        // channel , lobyte/higybyte, square wave
        #[expect(clippy::unusual_byte_groupings)]
        cmd.write(0b00_11_011_0);

        // LSB the MSB
        data.write((divisor & 0xFF) as u8);
        data.write((divisor >> 8) as u8);
    }
}

pub fn pit_tick_interrupt_handler() {
    let _num = super::MS_COUNT.fetch_add(1, core::sync::atomic::Ordering::AcqRel);
    // use crate::serial_println;
    // serial_println!("n: {num}");
}
