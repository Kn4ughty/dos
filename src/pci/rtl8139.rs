// http://realtek.info/pdf/rtl8139d.pdf
// https://wiki.osdev.org/RTL8139

use alloc::format;
use alloc::string::String;
use core::fmt::Write;

use crate::port::Port;
use crate::println;

const VENDOR_ID: u16 = 0x10EC;
const DEVICE_ID: u16 = 0x8139;

// pub struct MACAddress {
//     pub mac: [Port<u8>; 6],
// }

// impl core::fmt::Debug for MACAddress {
//     fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
//         for (i, seg) in self.mac.iter().enumerate() {
//             f.write_fmt(format_args!("{:02X}", seg.read()))?;
//
//             if i != 7 {
//                 f.write_char(':')?;
//             }
//         }
//         Ok(())
//     }
// }

// For the source of these offsets, see http://realtek.info/pdf/rtl8139d.pdf § Register Descriptions
#[expect(unused)]
#[derive(Debug)]
pub struct RTL8139 {
    mac: [Port<u8>; 6],
    // Receive (Rx) buffer Start Address. RBSTART
    receive_buffer_start: Port<u32>,
    command_reg: Port<u8>,
    config0: Port<u8>,
    config1: Port<u8>,
}

impl RTL8139 {
    /// IO base comes from PCI configuration, and is the base address, which is a port not a
    /// memory location
    #[must_use]
    pub fn new(io_base: u16) -> RTL8139 {
        RTL8139 {
            mac: [
                #[expect(clippy::identity_op)]
                Port::new(io_base + 0x00),
                Port::new(io_base + 0x01),
                Port::new(io_base + 0x02),
                Port::new(io_base + 0x03),
                Port::new(io_base + 0x04),
                Port::new(io_base + 0x05),
            ],
            receive_buffer_start: Port::new(io_base + 0x30),
            command_reg: Port::new(io_base + 0x37),
            config0: Port::new(io_base + 0x51),
            config1: Port::new(io_base + 0x52),
        }
    }

    pub fn init(&mut self) {
        // Power on
        unsafe {
            self.config1.write(0x0);
        }

        // Software reset
        unsafe {
            self.command_reg.write(0x10);
            println!("Waiting for RTL8139 reset to finish");
            while (self.command_reg.read() & 0x10) != 0 {}
        }
    }

    pub fn get_mac(&mut self) -> String {
        let mut out = String::new();
        for (i, seg) in &mut self.mac.iter_mut().enumerate() {
            _ = write!(out, "{:02X}", unsafe { seg.read() });
            if i != 5 {
                _ = write!(out, ":");
            }
        }
        out
    }

    #[must_use]
    pub const fn vendor_id() -> u16 {
        VENDOR_ID
    }

    #[must_use]
    pub const fn device_id() -> u16 {
        DEVICE_ID
    }
}
