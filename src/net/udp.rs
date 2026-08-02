use super::socket::Port;
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug)]
pub struct UdpPacketHeader {
    pub src_port: Port,
    pub dst_port: Port,
    pub length: u16,
    pub checksum: u16,
}

pub struct UdpPacket<'a> {
    header: UdpPacketHeader,
    data: &'a [u8],
}

impl UdpPacketHeader {
    fn to_bytes(self) -> [u8; 8] {
        let mut out = [0u8; 8];
        out[0..2].copy_from_slice(&self.src_port.0.to_be_bytes());
        out[2..4].copy_from_slice(&self.dst_port.0.to_be_bytes());
        out[4..6].copy_from_slice(&self.length.to_be_bytes());
        out[6..8].copy_from_slice(&self.checksum.to_be_bytes());

        out
    }
}

impl UdpPacketHeader {
    pub fn from_bytes(bytes: [u8; 8]) -> Self {
        UdpPacketHeader {
            src_port: u16::from_be_bytes(bytes[0..2].try_into().unwrap()).into(),
            dst_port: u16::from_be_bytes(bytes[2..4].try_into().unwrap()).into(),
            length: u16::from_be_bytes(bytes[4..6].try_into().unwrap()),
            checksum: u16::from_be_bytes(bytes[6..8].try_into().unwrap()),
        }
    }
}

impl<'a> UdpPacket<'a> {
    /// Creates a new udp packet with checksum set to 0
    // idea. use socket handle for src port to prove ownership
    #[must_use]
    pub fn new(src_port: Port, dst_port: Port, data: &'a [u8]) -> Self {
        let header = UdpPacketHeader {
            src_port,
            dst_port,
            length: u16::try_from(data.len() + 8).unwrap(),
            checksum: 0,
        };

        UdpPacket { header, data }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // Header is 8 bytes
        let mut joined: Vec<u8> = Vec::with_capacity(8 + self.data.len());
        joined.extend_from_slice(&self.header.to_bytes());
        joined.extend_from_slice(self.data);

        // checksum is not calculated because it is optional and easier

        joined
    }
}
