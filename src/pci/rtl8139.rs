// http://realtek.info/pdf/rtl8139d.pdf
// https://wiki.osdev.org/RTL8139

use alloc::string::String;
use alloc::vec::Vec;
use bitflags::bitflags;
use conquer_once::spin::OnceCell;
use core::fmt::Write;
use core::usize;

use crate::mem::{self, phys::PhysBuf};
use crate::port::Port;
use crate::println;
use crate::spinlock::Mutex;

pub static RTL: OnceCell<Mutex<RTL8139>> = OnceCell::uninit();

pub fn irq_handler() {
    if let Ok(rtl) = RTL.try_get()
        && let Some(mut rtl) = rtl.try_lock()
    {
        rtl.handle_interrupt();
    } else {
        println!("WARNING: irq_handler failed to lock RTL");
    }
}

pub const VENDOR_ID: u16 = 0x10EC;
// sadly the device id is not 8139
pub const DEVICE_ID: u16 = 0x8139;

const RX_BUFFER_PAD: usize = 16;
const RX_BUFFER_LEN: usize = 8192;

// Receive status register flags
bitflags! {
    #[derive(Debug)]
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
#[derive(Debug)]
pub struct Ports {
    pub mac: [Port<u8>; 6],
    pub tx_status0: Port<u32>,
    pub tx_start_addr0: Port<u32>,
    /// Receive (Rx) buffer Start Address. RBSTART
    pub receive_buffer_start: Port<u32>,
    pub command_reg: Port<u8>,
    /// Current address of packet read
    pub capr: Port<u16>,
    /// Reflects total received byte count in rx buffer
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
            tx_status0: Port::new(io_base + 0x10),
            tx_start_addr0: Port::new(io_base + 0x20),
            receive_buffer_start: Port::new(io_base + 0x30),
            command_reg: Port::new(io_base + 0x37),
            capr: Port::new(io_base + 0x38),
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
    tx_buf: PhysBuf,
    rx_offset: usize,
}

impl RTL8139 {
    #[must_use]
    pub fn new(io_base: u16) -> Self {
        Self {
            ports: Ports::new(io_base),
            rx_buf: PhysBuf::new(RX_BUFFER_LEN + RX_BUFFER_PAD),
            tx_buf: PhysBuf::new(2048),
            rx_offset: 0,
        }
    }

    pub fn init(&mut self) {
        println!("RTL8139 init.");
        // Power on
        unsafe {
            self.ports.config1.write(0x0);
        }

        // Software reset
        unsafe {
            self.ports.command_reg.write(0x10);
            // is a memfence needed here?
            println!("Waiting for RTL8139 reset to finish");
            let mut timeout = 0u32;
            while (self.ports.command_reg.read() & 0x10) != 0 {
                core::hint::spin_loop();
                timeout += 1;
                if timeout % 10_001 == 0 {
                    println!("more than 10k. Timeout possible. {:?}", timeout);
                }
            }
        }

        let rx_addr: u32 = self.rx_buf.addr().try_into().expect("rx_buf less than u32");
        println!("rx_buf phys addr: {:#010x}", rx_addr);
        unsafe {
            self.ports.receive_buffer_start.write(rx_addr);
            println!(
                "rx_buf readback: {:#010x}",
                self.ports.receive_buffer_start.read()
            );
        }

        // Setup interrupts
        use InterruptMask as IM;
        let imr_val = (IM::ROK | IM::TOK).bits();
        println!("Writing IMR: {:#06x}", imr_val);
        unsafe {
            self.ports.interrupt_mask.write(imr_val);
            println!("IMR readback: {:#06x}", self.ports.interrupt_mask.read());
        }

        // Configure receive buffer
        use ReceiveConfiguration as RC;
        let rcr_val = (RC::AB | RC::AM | RC::APM | RC::AR | RC::AAP | RC::WRAP).bits();
        println!("Writing RCR: {:#010x}", rcr_val);
        unsafe {
            self.ports.rx_config.write(rcr_val);
            println!("RCR readback: {:#010x}", self.ports.rx_config.read());
        }

        // Enable receive and transmitter
        unsafe {
            //  set RE and TE bits high
            self.ports.command_reg.write(0x0c);
            println!(
                "command_reg after enable: {:#04x}",
                self.ports.command_reg.read()
            );
        }

        println!("RTL8139 init completed");
    }

    pub fn register_interrupts(self: RTL8139, interrupt_line: u8) {
        RTL.try_init_once(move || Mutex::new(self))
            .expect("RTL8139 device already registed");

        crate::interrupts::set_irq_handler(interrupt_line, irq_handler);
        crate::interrupts::clear_irq_mask(interrupt_line);
    }

    pub fn send_arp(&mut self) {
        // ARP packet: who has 10.0.2.2? tell 10.0.2.15
        // (QEMU's default gateway is 10.0.2.2, guest is 10.0.2.15)
        let mut packet = [0u8; 42];

        // Ethernet header
        packet[0..6].copy_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]); // dst: broadcast
        let mac = self.ports.get_mac();
        packet[6..12].copy_from_slice(&mac); // src: our mac
        packet[12..14].copy_from_slice(&[0x08, 0x06]); // ethertype: ARP

        // ARP body
        packet[14..16].copy_from_slice(&[0x00, 0x01]); // hardware type: ethernet
        packet[16..18].copy_from_slice(&[0x08, 0x00]); // protocol type: IPv4
        packet[18] = 6; // hardware size
        packet[19] = 4; // protocol size
        packet[20..22].copy_from_slice(&[0x00, 0x01]); // opcode: request
        packet[22..28].copy_from_slice(&mac); // sender mac
        packet[28..32].copy_from_slice(&[10, 0, 2, 15]); // sender IP: 10.0.2.15
        packet[32..38].copy_from_slice(&[0, 0, 0, 0, 0, 0]); // target mac: unknown
        packet[38..42].copy_from_slice(&[10, 0, 2, 2]); // target IP: 10.0.2.2 (QEMU gateway)

        self.send_packet(&packet);
    }

    pub fn send_packet(&mut self, data: &[u8]) {
        // use crate::mem::virt_to_phys;
        // use x86_64::VirtAddr;
        // let virt = VirtAddr::from_ptr(data.as_ptr());
        // let phys = virt_to_phys(virt).expect("failed to translatge vaddr to phys addr");

        let len = data.len();
        self.tx_buf.buf[..len].copy_from_slice(data);

        let phys = self.tx_buf.addr();
        println!("send_packet: phys={:#010x}, len={}", phys, len);
        unsafe {
            self.ports
                .tx_start_addr0
                .write(u32::try_from(phys).expect("addr fits"));
            let readback = self.ports.tx_start_addr0.read();
            println!("TSAD0 readback: {:#010x}", readback);

            self.ports.tx_status0.write(data.len() as u32 & 0x1FFF);
            let status = self.ports.tx_status0.read();
            println!("TSD0 after write: {:#010x}", status);
        }
    }

    pub fn receive_packet(&mut self) -> Option<Vec<u8>> {
        let cmd = unsafe { self.ports.command_reg.read() };
        if (cmd & 1) == 1 {
            return None;
        }

        let offset = self.rx_offset;

        // //  offset address of packet start
        // let offset = (unsafe { self.ports.capr.read() } as usize + RX_BUFFER_PAD) % RX_BUFFER_LEN;

        let header = ReceiveStatus::from_bits_retain(u16::from_le_bytes(
            self.rx_buf.buf[offset..offset + 2].try_into().unwrap(),
        ));

        if !header.contains(ReceiveStatus::ROK) {
            println!("WARNING: Error receiving packet! {:?}", header);
            // need to cleanup

            return None;
        }
        let pkt_length = u16::from_le_bytes(
            self.rx_buf.buf[(offset + 2)..(offset + 4)]
                .try_into()
                .unwrap(),
        ) as usize;

        let out = self.rx_buf.buf[(offset + 4)..(offset + pkt_length)].to_vec();

        // We must advance capr to tell hardware we are done with this region of buffer

        // let capr = unsafe { self.ports.capr.read() };

        self.rx_offset = mem::align_up(offset + pkt_length + 4, 4) % RX_BUFFER_LEN;

        unsafe {
            self.ports
                .capr
                .write((self.rx_offset as u16).wrapping_sub(RX_BUFFER_PAD as u16));
        }

        Some(out)
    }

    pub fn handle_interrupt(&mut self) {
        let status = unsafe { self.ports.interrupt_status.read() };

        if status == 0 {
            return;
        }

        unsafe {
            self.ports.interrupt_status.write(status);
        }

        if (status & InterruptMask::ROK.bits()) != 0 {
            println!("Receiving packet");
            if let Some(packet) = self.receive_packet() {
                crate::task::network::push_packet(packet);
            }
        }

        if (status & InterruptMask::TOK.bits()) != 0 {
            println!("Packet transmitted successfully!");
        }

        if (status & InterruptMask::RER.bits()) != 0 {
            println!("Receive error occured!");
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
