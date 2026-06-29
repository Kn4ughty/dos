use alloc::vec::Vec;
use core::net::Ipv4Addr;

use super::{Interface, ones_complement_checksum};

pub struct Port(u16);

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
    pub fn new(
        _dst_ip: Ipv4Addr,
        src_port: Port,
        dst_port: Port,
        data: &'a [u8],
        _interface: Interface,
    ) -> Self {
        let header = UdpPacketHeader {
            src_port,
            dst_port,
            length: u16::try_from(data.len() + 8).unwrap(),
            checksum: 0,
        };

        // let mut full_header = [0u8; 20];
        // full_header[0..4].copy_from_slice(&src_ip.to_bits().to_be_bytes());
        // full_header[4..8].copy_from_slice(&dst_ip.to_bits().to_be_bytes());
        // full_header[9] = super::ip::IPProtocol::Udp as u8;
        // full_header[10..12].copy_from_slice(&data.len().to_be_bytes());
        // full_header[12..20].copy_from_slice(&header.to_bytes());
        //
        // let mut full = [0u8; 20 + data.len()];
        // full[0..20].copy_from_slice(&full_header);
        // full[20..].copy_from_slice(&data);
        //
        // ones_complement_checksum(full_header);
        //
        UdpPacket { header, data }
    }
}
