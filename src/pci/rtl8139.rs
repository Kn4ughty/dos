// http://realtek.info/pdf/rtl8139d.pdf
// https://wiki.osdev.org/RTL8139

use alloc::string::String;
use bitflags::bitflags;
use core::fmt::Write;

use crate::mem::phys::PhysBuf;
use crate::port::Port;
use crate::println;

pub const VENDOR_ID: u16 = 0x10EC;
// sadly the device id is not 8139
pub const DEVICE_ID: u16 = 0x8139;

const RX_BUFFER_PAD: usize = 16;
const RX_BUFFER_LEN: usize = 8192;

// Receive status register flags
bitflags! {
    struct ReceiveStatus: u16 {
        /// Receive Ok
        const ROK = 1;
        /// Frame alignment erro
        const FAE = 1 << 1;
        /// CRC error
        const CRC = 1 << 2;
        /// Long packet. Packet exceeds 4k bytes
        const LONG = 1 << 3;
        /// Packet is smaller than 64 bytes
        const RUNT = 1 << 4;
        /// Invalid symbol error
        const ISE = 1 << 5;
        /// Broadcase address received
        const BAR = 1 << 13;
        /// Physical address matched
        const PAM = 1 << 14;
        /// Multicast address received
        const MAR = 1 << 15;
    }
}

bitflags! {
    struct TransmitStatus: u16 {
        /// the rtl8139d sets this bit to 1 when Tx DMA op is completed
        const OWN = 1 << 13;
        /// Transmit ok
        const TOK = 1 << 15;
    }
}

bitflags! {
    struct InterruptMask: u16 {
        /// Recieve Ok
        const ROK = 1 << 0;
        /// Recieve error
        const RER = 1 << 1;
        /// Transmit Ok
        const TOK = 1 << 2;
        /// Transmission Error
        const TER = 1 << 3;
        /// rx buffer overflow
        const RXOVW = 1 << 4;
        const FOVW = 1 << 6;
    }
}

bitflags! {
    struct ReceiveConfiguration: u32 {
        /// Accept all packets
        const AAP = 1 << 0;
        /// Accept physical match packetsj
        const APM = 1 << 1;
        /// Accept multicast packets
        const AM = 1 << 2;
        /// Accept broadcast packets
        const AB = 1 << 3;
        /// Accept runt
        const AR = 1 << 4;
        /// Accept Error Packet
        const AER = 1 << 5;
        /// If 1, rtl will write past end of buffer.
        /// If 0, buffer will be like a ringbuffer.
        const WRAP = 1 << 7;
    }
}

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
struct Ports {
    pub mac: [Port<u8>; 6],
    // Receive (Rx) buffer Start Address. RBSTART
    pub receive_buffer_start: Port<u32>,
    pub command_reg: Port<u8>,
    // Reflects total received byte count in rx buffer
    pub current_buffer_address: Port<u16>,
    pub interrupt_mask: Port<u16>,
    pub interrupt_status: Port<u16>,
    pub tx_config: Port<u32>,
    pub rx_config: Port<u32>,
    pub config0: Port<u8>,
    pub config1: Port<u8>,
}

impl Ports {
    /// IO base comes from PCI configuration, and is the base address, which is a port not a
    /// memory location
    #[must_use]
    pub fn new(io_base: u16) -> Ports {
        Ports {
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
            current_buffer_address: Port::new(io_base + 0x3A),
            interrupt_mask: Port::new(io_base + 0x3C),
            interrupt_status: Port::new(io_base + 0x3E),
            tx_config: Port::new(io_base + 0x40),
            rx_config: Port::new(io_base + 0x44),
            config0: Port::new(io_base + 0x51),
            config1: Port::new(io_base + 0x52),
        }
    }

    pub fn get_mac(&mut self) -> [u8; 6] {
        unsafe {
            [
                self.mac[0].read(),
                self.mac[1].read(),
                self.mac[2].read(),
                self.mac[3].read(),
                self.mac[4].read(),
                self.mac[5].read(),
            ]
        }
    }
}

pub struct RTL8139 {
    ports: Ports,
    rx_buf: PhysBuf,
}

impl RTL8139 {
    #[must_use]
    pub fn new(io_base: u16) -> Self {
        Self {
            ports: Ports::new(io_base),
            rx_buf: PhysBuf::new(RX_BUFFER_LEN + RX_BUFFER_PAD),
        }
    }

    pub fn init(&mut self) {
        // Power on
        unsafe {
            self.ports.config1.write(0x0);
        }

        // Software reset
        unsafe {
            self.ports.command_reg.write(0x10);
            // is a memfence needed here?
            println!("Waiting for RTL8139 reset to finish");
            while (self.ports.command_reg.read() & 0x10) != 0 {
                core::hint::spin_loop();
            }
        }

        let rx_addr: u32 = self.rx_buf.addr().try_into().expect("rx_buf less than u32");
        unsafe {
            self.ports.receive_buffer_start.write(rx_addr);
        }

        // Setup interrupts
        use InterruptMask as IM;
        unsafe { self.ports.interrupt_mask.write((IM::ROK | IM::TOK).bits()) }

        // Configure receive buffer
        use ReceiveConfiguration as RC;
        unsafe {
            self.ports
                .rx_config
                .write((RC::AB | RC::AM | RC::APM | RC::AR | RC::AAP | RC::WRAP).bits());
        }

        // Enable receive and transmitter
        unsafe {
            //  set RE and TE bits high
            self.ports.command_reg.write(0x0c);
        }
    }

    pub fn receive_packet(&mut self) -> Option<()> {
        let cmd = unsafe { self.ports.command_reg.read() };
        if (cmd & 1) == 1 {
            return None;
        }
        println!("cmd set!");

        // let cba = unsafe {
        //     self.ports.
        // }
        Some(())
    }

    pub fn handle_interrupt(&mut self) {
        let status = unsafe { self.ports.interrupt_status.read() };

        if status == 0 {
            return;
        }

        if (status & InterruptMask::ROK.bits()) != 0 {
            println!("ROK SET");
            self.receive_packet();
        }

        if (status & InterruptMask::TOK.bits()) != 0 {
            println!("Packet transmitted successfully!");
        }

        if (status & InterruptMask::RER.bits()) != 0 {
            println!("Receive error occured!");
        }

        unsafe {
            self.ports.interrupt_status.write(status);
        }
    }

    pub fn mac_string(&mut self) -> String {
        fmt_mac(&self.ports.get_mac())
    }
}

#[must_use]
pub fn fmt_mac(mac: &[u8; 6]) -> String {
    let mut out = String::new();
    for (i, seg) in &mut mac.iter().enumerate() {
        _ = write!(out, "{:02X}", seg);
        if i != 5 {
            _ = write!(out, ":");
        }
    }
    out
}
