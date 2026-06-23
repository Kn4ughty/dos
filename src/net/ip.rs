use super::{arp, ethernet};
use crate::tryfrom::tryfrom;
use bitflags::bitflags;
use core::{
    hash::{Hash, Hasher},
    net::Ipv4Addr,
};
use log::trace;

use super::{Interface, ones_complement_checksum};

pub async fn handle_packet(packet: &IPv4Packet<'_>, interface: Interface) {
    trace!("unfilted ip_packet_header: {:?}", packet.header);
    // TODO handle more ip's like loopback
    if packet.header.destination_address != interface.ip {
        return;
    }
    trace!("Decided to handle packet");

    match packet.header.protocol {
        IPProtocol::Icmp => {
            use super::icmp;
            icmp::handle_icmp(packet, interface).await;
        }
        IPProtocol::Tcp => {
            // todo
            trace!("Ignoring TCP packet");
        }
        IPProtocol::Udp => {
            // todo
            trace!("Ignoring UDP packet");
        }
    }
}

pub async fn send_ipv4_packet(packet: IPv4Packet<'_>, interface: Interface) {
    // Goal. Determine if a destination address is within the current subnet
    // if (mask & gateway) == (mask & destination_address)
    let on_same_subnet: bool = interface.subnet_mask & interface.gateway
        == interface.subnet_mask & packet.header.destination_address;

    let mut dest_ip_for_arp = packet.header.destination_address;
    if !on_same_subnet {
        dest_ip_for_arp = interface.gateway;
    }

    let Some(dest_mac) = arp::find_target(dest_ip_for_arp, &interface).await else {
        log::error!(
            "Unable to send ip packet {:?} \n No mac address found. Dropping",
            packet.header
        );

        return;
    };

    let bytes = packet.to_bytes();
    let ep = ethernet::EthernetPacket {
        destination: dest_mac,
        source: interface.mac,
        typ: ethernet::EtherType::IPv4,
        data: bytes.as_slice(),
    };
    super::send_frame(interface, ep.into()).await;
}

// see https://www.iana.org/assignments/protocol-numbers/protocol-numbers.xhtml
tryfrom! {
    #[repr(u8)]
    #[derive(Debug, Clone, Copy, Hash)]
    pub enum IPProtocol {
        Icmp = 0x01,
        Tcp = 0x06,
        Udp = 0x11,
    }, u8
}

#[derive(Debug)]
pub struct IPv4Header {
    // omg noo its not actually 4 bitss, im wasting memoryy 👁️👄👁️
    /// 4 bit version. For ipv4, this is always 4 (lol)
    version: u8,
    /// 4 bit header length. Length of the header in 32 bit words.
    /// Minimum is 5. We never use options so its always going to = 5
    ihl: u8,
    /// 6 bit Differentiated Services Code point
    dscp: u8,
    /// 2 bit Explicit Congestion notification
    ecn: u8,
    /// Total size of entire packet in bytes, including header and data
    total_length: u16,
    /// Used for identifying the group of fragments of a single IP datagram.
    identification: u16,
    /// 3 bit flag field
    flags: IPv4Flags,
    /// 13 bit Fragment Offset
    fragment_offset: u16,
    /// 8 bit Time to live. Specfied in seconds. In practice this is used as a hop count
    /// This is how traceroute works!
    ttl: u8,

    /// 8 bit protocol. Defines the next level protocol.
    /// See <https://en.wikipedia.org/wiki/List_of_IP_protocol_numbers>
    protocol: IPProtocol,

    /// 16 bit one's complement of all the 16 bit words in the header.
    header_checksum: u16,

    /// good old ip address we know and love
    pub source_address: Ipv4Addr,
    pub destination_address: Ipv4Addr,
}

impl TryFrom<[u8; 20]> for IPv4Header {
    type Error = IpError;
    fn try_from(v: [u8; 20]) -> Result<Self, Self::Error> {
        Ok(Self {
            version: (v[0] >> 4) & 0xF,
            ihl: {
                let len = v[0] & 0xF;
                if len != 5 {
                    log::warn!("IPv4 packet with wrong len. Keeping with warning.");
                }
                len
            },
            dscp: (v[1] >> 2) & 0b0000_1111,
            ecn: v[1] & 0b11,
            total_length: u16::from_be_bytes(v[2..=3].try_into().unwrap()),
            identification: u16::from_be_bytes(v[4..=5].try_into().unwrap()),
            flags: (IPv4Flags::from_bits_retain(v[6] >> 5)),
            fragment_offset: u16::from_be_bytes(v[6..=7].try_into().unwrap()) & 0x1FFF,
            ttl: v[8],
            protocol: IPProtocol::try_from(v[9]).map_err(|_| IpError::UnknownProtocol)?,
            header_checksum: u16::from_be_bytes(v[10..=11].try_into().unwrap()),
            source_address: Ipv4Addr::from_octets(v[12..=15].try_into().unwrap()),
            destination_address: Ipv4Addr::from_octets(v[16..=19].try_into().unwrap()),
        })
    }
}

impl Hash for IPv4Header {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash only the fields that make sense for identification
        self.identification.hash(state);
        self.source_address.hash(state);
        self.destination_address.hash(state);
        self.protocol.hash(state);
    }
}

bitflags! {
    #[derive(Debug)]
    struct IPv4Flags: u8 {
        /// Reserved
        const R = 1;
        /// Don't Fragment
        const DF = 1 << 1;
        /// More fragments
        const MF = 1 << 2;
    }
}

#[derive(Debug)]
pub enum IpError {
    PacketNotLongEnough,
    UnknownProtocol,
    DataTooLong,
}

#[derive(Debug)]
pub struct IPv4Packet<'a> {
    pub header: IPv4Header,
    pub data: &'a [u8],
}

impl<'a> TryFrom<&'a [u8]> for IPv4Packet<'a> {
    type Error = IpError;
    fn try_from(v: &'a [u8]) -> Result<Self, Self::Error> {
        if v.len() < 21 {
            return Err(IpError::PacketNotLongEnough);
        }

        Ok(IPv4Packet {
            header: IPv4Header::try_from(
                <&[u8] as TryInto<[u8; 20]>>::try_into(&v[0..20]).unwrap(),
            )?,
            data: &v[20..v.len()],
        })
    }
}

impl<'a> IPv4Packet<'a> {
    /// Includes checksum
    pub fn from_source_dest_and_data(
        source: Ipv4Addr,
        destination: Ipv4Addr,
        data: &'a [u8],
    ) -> Result<IPv4Packet<'a>, IpError> {
        let total_len = u16::try_from(20 + data.len()).map_err(|_| IpError::DataTooLong)?;

        if total_len > ethernet::ETHERNET_MTU {
            log::warn!(
                "Tried to create Ipv4Packet ({})bytes bigger than MTU ({})bytes",
                total_len,
                ethernet::ETHERNET_MTU
            );
            return Err(IpError::DataTooLong);
        }

        let p = IPv4Packet {
            header: IPv4Header {
                version: 4,
                ihl: 5,
                dscp: 0,
                ecn: 0,
                total_length: total_len,
                identification: 0xFEED,
                flags: IPv4Flags::DF,
                fragment_offset: 0,
                ttl: 20,
                protocol: IPProtocol::Icmp,
                header_checksum: 0,
                source_address: source,
                destination_address: destination,
            },
            data,
        };

        Ok(p.with_checksum())
    }
    pub fn to_bytes(&self) -> alloc::vec::Vec<u8> {
        let h = &self.header;
        let mut buf = alloc::vec![0u8; 20 + self.data.len()];

        buf[0] = (h.version << 4) | (h.ihl & 0xF);
        buf[1] = (h.dscp << 2) | (h.ecn & 0b11);
        buf[2..4].copy_from_slice(&h.total_length.to_be_bytes());
        buf[4..6].copy_from_slice(&h.identification.to_be_bytes());
        // Flags in top 3 bits, fragment offset in low 13
        let flags_and_offset: u16 =
            (u16::from(h.flags.bits()) << 13) | (h.fragment_offset & 0x1FFF);
        buf[6..8].copy_from_slice(&flags_and_offset.to_be_bytes());
        buf[8] = h.ttl;
        buf[9] = h.protocol as u8;
        buf[10..12].copy_from_slice(&h.header_checksum.to_be_bytes());
        buf[12..16].copy_from_slice(&h.source_address.octets());
        buf[16..20].copy_from_slice(&h.destination_address.octets());
        buf[20..].copy_from_slice(self.data);

        buf
    }

    /// Calculates new checksum for packet.
    /// Use after construction
    pub fn with_checksum(mut self) -> Self {
        self.header.header_checksum = 0;
        let bytes = self.to_bytes();
        self.header.header_checksum = ones_complement_checksum(&bytes[..20]);
        self
    }
}
