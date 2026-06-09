/// Programmable interval timer
// https://www.scs.stanford.edu/10wi-cs140/pintos/specs/8254.pdf
use crate::port::{Port, PortReadOnly};

const PIT_BASE_FREQUENCY: f64 = 1_193_182.0; // hz 

const CH0_PORT: u16 = 0x40;
const CMD_PORT: u16 = 0x43;

pub fn set_interval() {
    todo!()
}
