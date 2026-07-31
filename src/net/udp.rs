use alloc::vec::Vec;

use super::ones_complement_checksum;

#[derive(Clone, Copy, Debug)]
pub struct Port(u16);

impl From<u16> for Port {
    fn from(value: u16) -> Self {
        Port(value)
    }
}

#[derive(Clone, Copy, Debug)]
struct UdpPacketHeader {
    src_port: Port,
    dst_port: Port,
    length: u16,
    checksum: u16,
}

pub struct UdpPacket<'a> {
    header: UdpPacketHeader,
    data: &'a [u8],
}

impl UdpPacketHeader {
    fn to_bytes(&self) -> [u8; 8] {
        let mut out = [0u8; 8];
        out[0..2].copy_from_slice(&self.src_port.0.to_be_bytes());
        out[2..4].copy_from_slice(&self.dst_port.0.to_be_bytes());
        out[4..6].copy_from_slice(&self.length.to_be_bytes());
        out[6..8].copy_from_slice(&self.checksum.to_be_bytes());

        out
    }
}

impl<'a> UdpPacket<'a> {
    /// Creates a new udp packet with checksum set to 0
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

    /// Includes checksum calculation
    pub fn to_bytes(&self) -> Vec<u8> {
        // Header is 8 bytes
        let mut joined: Vec<u8> = Vec::with_capacity(8 + self.data.len());
        joined.extend_from_slice(&self.header.to_bytes());
        joined.extend_from_slice(self.data);

        // recalcuate checksum
        let checksum = ones_complement_checksum(joined.as_slice());
        let check_bytes = checksum.to_le_bytes();
        joined[6] = check_bytes[0];
        joined[7] = check_bytes[1];
        joined
    }
}
