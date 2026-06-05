use crate::net::ethernet::EtherType;
use crate::println;

use super::ethernet::MacAddress;
use core::convert::TryInto;

// pub fn send_arp() {
//     // ARP packet: who has 10.0.2.2? tell 10.0.2.15
//     // (QEMU's default gateway is 10.0.2.2, guest is 10.0.2.15)
//     let mut packet = [0u8; 42];
//
//     // Ethernet header
//     packet[0..6].copy_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]); // dst: broadcast
//
//     let mac = super::nic::rtl8139::RTL.get().unwrap().lock().get_mac();
//
//     packet[6..12].copy_from_slice(&mac); // src: our mac
//     packet[12..14].copy_from_slice(&[0x08, 0x06]); // ethertype: ARP
//
//     // ARP body
//     packet[14..16].copy_from_slice(&[0x00, 0x01]); // hardware type: ethernet
//     packet[16..18].copy_from_slice(&[0x08, 0x00]); // protocol type: IPv4
//     packet[18] = 6; // hardware size
//     packet[19] = 4; // protocol size
//     packet[20..22].copy_from_slice(&[0x00, 0x01]); // opcode: request
//     packet[22..28].copy_from_slice(&mac); // sender mac
//     packet[28..32].copy_from_slice(&[10, 0, 2, 15]); // sender IP: 10.0.2.15
//     packet[32..38].copy_from_slice(&[0, 0, 0, 0, 0, 0]); // target mac: unknown
//     packet[38..42].copy_from_slice(&[10, 0, 2, 2]); // target IP: 10.0.2.2 (QEMU gateway)
//
//     // self.send_packet(&packet);
//     super::nic::rtl8139::RTL
//         .get()
//         .unwrap()
//         .lock()
//         .send_packet(&packet);
// }

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
    sender_protocol_address: u32,
    /// In a request, this field is ignored.
    /// In a reply this field is used to indicate the address of the host that originated the ARP
    /// request
    target_hardware_address: MacAddress,
    /// ip address of intended receiver
    target_protocol_address: u32,
}

impl TryFrom<&[u8]> for ArpPacket {
    type Error = ArpError;
    fn try_from(v: &[u8]) -> Result<Self, Self::Error> {
        if v.len() < 28 {
            return Err(ArpError::PacketNotLongEnough);
        }

        #[expect(clippy::if_not_else, reason = "error first is clearer in this case")]
        #[expect(clippy::useless_conversion, reason = "Clearer")]
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
            sender_protocol_address: u32::from_be_bytes(v[14..=17].try_into().unwrap()),
            target_hardware_address: MacAddress::from(
                <&[u8] as TryInto<[u8; 6]>>::try_into(&v[18..=23]).unwrap(),
            ),
            target_protocol_address: u32::from_be_bytes(v[24..=27].try_into().unwrap()),
        })
    }
}

impl ArpPacket {
    pub fn new_arp_request(sender_mac: MacAddress, sender_ip: u32, target_ip: u32) -> ArpPacket {
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
    pub fn to_bytes(&self) -> [u8; 28] {
        let mut bytes = [0u8; 28];

        bytes[0..=1].copy_from_slice(&self.hardware_type.to_be_bytes());
        bytes[2..=3].copy_from_slice(&(self.protocol_type as u16).to_be_bytes());
        bytes[4] = self.hardware_length;
        bytes[5] = self.protocol_length;
        bytes[6..=7].copy_from_slice(&self.operation.to_be_bytes());
        bytes[8..=13].copy_from_slice(&self.sender_hardware_address.0);
        bytes[14..=17].copy_from_slice(&self.sender_protocol_address.to_be_bytes());
        bytes[18..=23].copy_from_slice(&self.target_hardware_address.0);
        bytes[24..=27].copy_from_slice(&self.target_protocol_address.to_be_bytes());

        bytes
    }
}
