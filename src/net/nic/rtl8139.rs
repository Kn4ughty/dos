// http://realtek.info/pdf/rtl8139d.pdf
// https://wiki.osdev.org/RTL8139

use alloc::vec::Vec;
use bitflags::bitflags;
use conquer_once::spin::OnceCell;
use log::{debug, error, trace};

use crate::mem::{self, phys::PhysBuf};
use crate::net::ethernet::EthernetFrame;
use crate::net::{EthernetDevice, ethernet::MacAddress};
use crate::port::Port;
use crate::sync::spinlock::Mutex;

pub static RTL: OnceCell<Mutex<RTL8139>> = OnceCell::uninit();

pub const VENDOR_ID: u16 = 0x10EC;
// sadly the device id is not 8139
pub const DEVICE_ID: u16 = 0x8139;

const RX_BUFFER_PAD: usize = 16;
const RX_BUFFER_WRAP_PAD: usize = 1500;
const RX_BUFFER_LEN: usize = 8192;

const TX_BUFFER_SIZE: usize = 2048;

pub fn irq_handler() {
    if let Ok(rtl) = RTL.try_get()
        && let Some(mut rtl) = rtl.try_lock()
    {
        rtl.handle_interrupt();
    } else {
        error!("WARNING: irq_handler failed to lock RTL");
    }
}

pub fn find_rtl() {
    log::debug!("Finding rtl");
    for bus in 0..=255 {
        for device in 0..=31 {
            for function in 0..=7 {
                let mut pci_device = crate::pci::PCIDevice::new(bus, device, function);
                #[expect(clippy::collapsible_if, reason = "future proofing")]
                if let Some(header) = pci_device.get_header() {
                    if let Some(mut rtl) = RTL8139::try_new(&header) {
                        log::debug!("Found rtl!");
                        log::debug!("{:#?}", header);
                        pci_device.enable_bus_mastering();

                        rtl.init();
                        // rtl.send_arp();
                        log::debug!("RTL mac address {:?}", rtl.get_mac());
                        rtl.register_interrupts(header.interrupt_line);
                        return;
                    }
                }
            }
        }
    }
    log::warn!("Could not find RTL");
}

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
    struct TransmitStatus: u32 {
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

// For the source of these offsets, see http://realtek.info/pdf/rtl8139d.pdf § Register Descriptions
#[derive(Debug)]
struct Ports {
    pub mac: [Port<u8>; 6],
    pub tx_status: [Port<u32>; 4],
    pub tx_start_addr: [Port<u32>; 4],
    pub receive_buffer_start: Port<u32>,
    pub command_reg: Port<u8>,
    /// Current address of packet read
    pub capr: Port<u16>,
    /// Reflects total received byte count in rx buffer
    pub _current_buffer_address: Port<u16>,
    pub interrupt_mask: Port<u16>,
    pub interrupt_status: Port<u16>,
    pub _tx_config: Port<u32>,
    pub rx_config: Port<u32>,
    pub _config0: Port<u8>,
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
            tx_status: [
                Port::new(io_base + 0x10),
                Port::new(io_base + 0x14),
                Port::new(io_base + 0x18),
                Port::new(io_base + 0x1C),
            ],
            tx_start_addr: [
                Port::new(io_base + 0x20),
                Port::new(io_base + 0x24),
                Port::new(io_base + 0x28),
                Port::new(io_base + 0x2C),
            ],
            // tx_status0: Port::new(io_base + 0x10),
            // tx_start_addr0: Port::new(io_base + 0x20),
            receive_buffer_start: Port::new(io_base + 0x30),
            command_reg: Port::new(io_base + 0x37),
            capr: Port::new(io_base + 0x38),
            _current_buffer_address: Port::new(io_base + 0x3A),
            interrupt_mask: Port::new(io_base + 0x3C),
            interrupt_status: Port::new(io_base + 0x3E),
            _tx_config: Port::new(io_base + 0x40),
            rx_config: Port::new(io_base + 0x44),
            _config0: Port::new(io_base + 0x51),
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
    tx_desc: usize,
}

impl RTL8139 {
    #[must_use]
    pub fn try_new(pci_header: &crate::pci::PCIDeviceHeader) -> Option<Self> {
        if pci_header.vendor_id != VENDOR_ID || pci_header.device_id != DEVICE_ID {
            return None;
        }

        let io_base = {
            let base = pci_header.base_addr0;
            // These do not return options, because they indicate something has gone quite wrong.
            // Something with matching vid and did should absolutely pass these asserts
            assert_eq!(
                base & 0x1,
                1,
                "must be odd to indicate that base_addr is a port"
            );
            assert!(base < u32::from(u16::MAX), "Must be a valid port");
            (pci_header.base_addr0 & 0xFFFC) as u16
        };

        Some(Self {
            ports: Ports::new(io_base),
            rx_buf: PhysBuf::new(RX_BUFFER_LEN + RX_BUFFER_PAD + RX_BUFFER_WRAP_PAD),
            tx_buf: PhysBuf::new(TX_BUFFER_SIZE * 4),
            rx_offset: 0,
            tx_desc: 0,
        })
    }

    pub fn init(&mut self) {
        debug!("RTL8139 init.");
        // Power on
        unsafe {
            self.ports.config1.write(0x0);
        }

        // Software reset
        unsafe {
            self.ports.command_reg.write(0x10);
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
        let imr_val = (IM::ROK | IM::TOK).bits();
        unsafe {
            self.ports.interrupt_mask.write(imr_val);
        }

        // Configure receive buffer
        use ReceiveConfiguration as RC;
        let rcr_val = (RC::AB | RC::AM | RC::APM | RC::AR | RC::AAP | RC::WRAP).bits();
        unsafe {
            self.ports.rx_config.write(rcr_val);
        }

        // Enable receive and transmitter
        unsafe {
            //  set RE and TE bits high
            self.ports.command_reg.write(0x0c);
        }

        debug!("RTL8139 init completed");
    }

    pub fn register_interrupts(self: RTL8139, interrupt_line: u8) {
        RTL.try_init_once(move || Mutex::new(self))
            .expect("RTL8139 device already registed");

        crate::interrupts::set_irq_handler(interrupt_line, irq_handler);
        crate::interrupts::clear_irq_mask(interrupt_line);
    }

    pub fn handle_interrupt(&mut self) {
        loop {
            let status = unsafe { self.ports.interrupt_status.read() };

            if status == 0 {
                return;
            }

            unsafe {
                self.ports.interrupt_status.write(status);
            }

            if (status & InterruptMask::ROK.bits()) != 0 {
                trace!("Receiving packet");
                while let Some(packet) = self.receive_packet() {
                    super::super::push_packet(packet);
                }
            }

            if (status & InterruptMask::TOK.bits()) != 0 {
                trace!("Packet transmitted successfully!");
                super::super::notify_tx_complete();
            }

            if (status & InterruptMask::RER.bits()) != 0 {
                trace!("Receive error occured!");
            }
        }
    }

    pub fn get_mac(&mut self) -> MacAddress {
        MacAddress::from(self.ports.get_mac())
    }
}

impl EthernetDevice for RTL8139 {
    fn send_packet(&mut self, packet: &EthernetFrame) {
        let packet = packet.as_bytes();
        let len = packet.len();
        assert!(len <= TX_BUFFER_SIZE, "too much data");

        let desc = self.tx_desc;
        self.tx_desc = (self.tx_desc + 1) % 4;

        let tx_status = &mut self.ports.tx_status[desc];
        let tx_address = &mut self.ports.tx_start_addr[desc];

        let offset = desc * TX_BUFFER_SIZE;

        assert!(
            TransmitStatus::from_bits_retain(unsafe { tx_status.read() })
                .contains(TransmitStatus::OWN),
            "TX descriptor {desc} still owned by RTL."
        );

        self.tx_buf.buf[offset..offset + len].copy_from_slice(packet);
        let phys = self.tx_buf.addr() + (offset as u64);

        trace!("send_packet: phys={:#010x}, len={}", phys, len);
        unsafe {
            tx_address.write(u32::try_from(phys).expect("addr ffits"));
            tx_status.write(u32::try_from(len).expect("too much data") & 0x1FFF);
        }
    }

    fn receive_packet(&mut self) -> Option<Vec<u8>> {
        let cmd = unsafe { self.ports.command_reg.read() };
        if (cmd & 1) == 1 {
            return None;
        }

        let offset = self.rx_offset;

        let header = ReceiveStatus::from_bits_retain(u16::from_le_bytes(
            self.rx_buf.buf[offset..offset + 2].try_into().unwrap(),
        ));

        if !header.contains(ReceiveStatus::ROK) {
            error!("WARNING: Error receiving packet! {:?}", header);
            // May need to still advance capr.
            // This is a future problem. We will find out when an error happens.
            // yk what, so that we MUST handle it in future ill just panic
            panic!("Error receiving packet");

            // return None;
        }
        let pkt_length = u16::from_le_bytes(
            self.rx_buf.buf[(offset + 2)..(offset + 4)]
                .try_into()
                .unwrap(),
        ) as usize;

        // We allocate a new vec bc if we just pass around a reference to the data,
        // then that could be modified by hardware at any moment! That would be very bad.
        let out = self.rx_buf.buf[(offset + 4)..(offset + pkt_length)].to_vec();

        // We must advance capr to tell hardware we are done with this region of buffer
        self.rx_offset = mem::align_up(offset + pkt_length + 4, 4) % RX_BUFFER_LEN;
        unsafe {
            self.ports.capr.write(
                (u16::try_from(self.rx_offset).expect("RX_BUFFER_LEN must be less than u16::MAX"))
                    .wrapping_sub(u16::try_from(RX_BUFFER_PAD).expect("Padding must fit")),
            );
        }

        Some(out)
    }
}
