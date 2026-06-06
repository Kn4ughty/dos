use bitflags::bitflags;
use core::net::Ipv4Addr;

struct IPv4Header {
    // omg noo its not actually 4 bitss, im wasting memoryy 👁️👄👁️
    /// 4 bit version. For ipv4, this is always 4 (lol)
    version: u8,
    /// 4 bit header length. Length of the header in 32 bit words.
    /// Minimum is 5.
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
    protocol: u8,

    /// 16 bit one's complement of all the 16 bit words in the header.
    header_checksum: u16,

    /// good old ip address we know and love
    source_address: Ipv4Addr,
    destination_address: Ipv4Addr,
}

impl From<[u8; 20]> for IPv4Header {
    fn from(v: [u8; 20]) -> Self {
        Self {
            version: (v[0] >> 4) & 0xF,
            ihl: v[0] & 0xF,
            dscp: (v[1] >> 2) & 0b0000_1111,
            ecn: v[1] & 0b11,
            total_length: u16::from_be_bytes(v[2..=3].try_into().unwrap()),
            identification: u16::from_be_bytes(v[4..=5].try_into().unwrap()),
            flags: (IPv4Flags::from_bits_retain(v[6] >> 5)),
            fragment_offset: u16::from_be_bytes(v[6..=7].try_into().unwrap()) & 0b0011_1111,
            ttl: v[8],
            protocol: v[9],
            header_checksum: u16::from_be_bytes(v[10..=11].try_into().unwrap()),
            source_address: Ipv4Addr::from_octets(v[12..=15].try_into().unwrap()),
            destination_address: Ipv4Addr::from_octets(v[16..=19].try_into().unwrap()),
        }
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

pub struct IPv4Packet<'a> {
    header: IPv4Header,
    data: &'a [u8],
}
