use crate::net::Interface;
use crate::net::ethernet::{EtherType, EthernetPacket};
use crate::println;
use crate::spinlock::Mutex;
use core::net::Ipv4Addr;

use super::ethernet::MacAddress;
use core::convert::TryInto;
use hashbrown::HashMap;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref ARP_TABLE: Mutex<HashMap<Ipv4Addr, MacAddress>> = Mutex::new(HashMap::new());
}

pub async fn handle_arp_incoming(p: &ArpPacket, interface: &Interface) {
    println!("Handling arp");
    match p.operation {
        1 => {
            // Request
            println!("arp_requst: {:?}", p);

            // Ignore packets not addressed to ourselves
            if p.target_protocol_address != interface.config.ip {
                return;
            }

            // Learn sender's mac while we can
            ARP_TABLE
                .lock()
                .insert(p.sender_protocol_address, p.sender_hardware_address);

            println!("Arp matches, sending reply");
            let arp = ArpPacket::new_arp_reply(
                interface.config.mac,
                interface.config.ip,
                p.sender_hardware_address,
                p.sender_protocol_address,
            );

            let arp_bytes = arp.to_bytes();
            let ep = EthernetPacket {
                destination: p.sender_hardware_address,
                source: interface.config.mac,
                typ: EtherType::Arp,
                data: &arp_bytes,
            };

            super::send_frame(*interface, ep.into()).await;
        }
        2 => {
            // Reply
            ARP_TABLE
                .lock()
                .insert(p.sender_protocol_address, p.sender_hardware_address);
        }
        unknown => {
            println!("Unknown ARP operation: {unknown}");
        }
    }
    println!("table: {:?}", ARP_TABLE.lock());
}

pub enum ArpError {
    PacketNotLongEnough,
    HardwareLengthNot6,
    HardwareTypeNotEthernet,
    ProtocolTypeNotIPv4,
    ProtocolLenNot4,
    UnknownProtocolType,
}

/// Must be encapsulated inside an `EthernetPacket`
#[derive(Debug)]
pub struct ArpPacket {
    /// Value of 1 indicates Ethernet
    hardware_type: u16,
    /// for IPv4, value is 0x0800. See <https://en.wikipedia.org/wiki/EtherType>
    protocol_type: EtherType,
    /// Length of the hardware address
    /// In this case always 6
    hardware_length: u8,
    /// For ipv4, always 4 bytes
    protocol_length: u8,

    /// 1 is request, 2 is reply
    operation: u16,

    /// Indicate the address of the host sending the request.
    /// In a reply, indicates the address of the host that the request was looking for
    sender_hardware_address: MacAddress,

    /// ip address of the sender
    sender_protocol_address: Ipv4Addr,

    /// In a request, this field is ignored.
    /// In a reply this field is used to indicate the address of the host that originated the ARP
    /// request
    target_hardware_address: MacAddress,

    /// ip address of intended receiver
    target_protocol_address: Ipv4Addr,
}

impl TryFrom<&[u8]> for ArpPacket {
    type Error = ArpError;
    fn try_from(v: &[u8]) -> Result<Self, Self::Error> {
        if v.len() < 28 {
            return Err(ArpError::PacketNotLongEnough);
        }

        #[expect(clippy::if_not_else, reason = "error first is clearer in this case")]
        Ok(ArpPacket {
            hardware_type: {
                let t = u16::from_be_bytes(v[0..=1].try_into().unwrap());
                if t != 1 {
                    Err(ArpError::HardwareTypeNotEthernet)
                } else {
                    Ok(t)
                }
            }?,
            protocol_type: {
                let prot = u16::from_be_bytes(v[2..=3].try_into().unwrap());
                let prot = EtherType::try_from(prot).map_err(|_| ArpError::UnknownProtocolType)?;
                if prot as u16 != EtherType::IPv4 as u16 {
                    Err(ArpError::ProtocolTypeNotIPv4)
                } else {
                    Ok(prot)
                }
            }?,
            hardware_length: {
                let len = v[4];
                if len != 6 {
                    println!("MAC ADDRESS IS NOT len 6");
                    Err(ArpError::HardwareLengthNot6)
                } else {
                    Ok(len)
                }
            }?,
            protocol_length: {
                let prot_len = v[5];
                if prot_len != 4 {
                    println!("WARNING. Bad prot_length");
                    Err(ArpError::ProtocolLenNot4)
                } else {
                    Ok(prot_len)
                }
            }?,
            operation: u16::from_be_bytes(v[6..=7].try_into().unwrap()),
            sender_hardware_address: MacAddress::from(
                <&[u8] as TryInto<[u8; 6]>>::try_into(&v[8..=13]).unwrap(),
            ),
            sender_protocol_address: Ipv4Addr::from_octets(v[14..=17].try_into().unwrap()),
            target_hardware_address: MacAddress::from(
                <&[u8] as TryInto<[u8; 6]>>::try_into(&v[18..=23]).unwrap(),
            ),
            target_protocol_address: Ipv4Addr::from_octets(v[24..=27].try_into().unwrap()),
        })
    }
}

impl ArpPacket {
    pub fn new_arp_request(
        sender_mac: MacAddress,
        sender_ip: Ipv4Addr,
        target_ip: Ipv4Addr,
    ) -> ArpPacket {
        ArpPacket {
            hardware_type: 1,
            protocol_type: EtherType::IPv4,
            hardware_length: 6,
            protocol_length: 4,
            operation: 1, // requets
            sender_hardware_address: sender_mac,
            sender_protocol_address: sender_ip,
            target_hardware_address: [0u8; 6].into(),
            target_protocol_address: target_ip,
        }
    }

    pub fn new_arp_reply(
        sender_mac: MacAddress,
        sender_ip: Ipv4Addr,
        target_mac: MacAddress,
        target_ip: Ipv4Addr,
    ) -> ArpPacket {
        ArpPacket {
            hardware_type: 1,
            protocol_type: EtherType::IPv4,
            hardware_length: 6,
            protocol_length: 4,
            operation: 2, // reply
            sender_hardware_address: sender_mac,
            sender_protocol_address: sender_ip,
            target_hardware_address: target_mac,
            target_protocol_address: target_ip,
        }
    }

    pub fn to_bytes(&self) -> [u8; 28] {
        let mut bytes = [0u8; 28];

        bytes[0..=1].copy_from_slice(&self.hardware_type.to_be_bytes());
        bytes[2..=3].copy_from_slice(&(self.protocol_type as u16).to_be_bytes());
        bytes[4] = self.hardware_length;
        bytes[5] = self.protocol_length;
        bytes[6..=7].copy_from_slice(&self.operation.to_be_bytes());
        bytes[8..=13].copy_from_slice(&self.sender_hardware_address.0);
        bytes[14..=17].copy_from_slice(&self.sender_protocol_address.octets());
        bytes[18..=23].copy_from_slice(&self.target_hardware_address.0);
        bytes[24..=27].copy_from_slice(&self.target_protocol_address.octets());

        bytes
    }
}
