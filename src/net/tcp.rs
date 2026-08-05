use core::net::Ipv4Addr;

use alloc::vec::Vec;
use crossbeam_queue::ArrayQueue;

use crate::net::{
    self,
    ip::{self, IpError},
    socket,
};
use bitflags::bitflags;
use socket::Port;

/// Manages all connections for a particular tcp socket
pub struct TcpSocket<'a> {
    pub port: Port,
    binding_addresss: Ipv4Addr,
    connections: Vec<TcpConnection<'a>>,
}

impl<'a> TcpSocket<'a> {
    pub fn new(port: Port, binding_addresss: Ipv4Addr) -> TcpSocket<'a> {
        TcpSocket {
            port,
            binding_addresss,
            connections: Vec::new(),
        }
    }

    // maybe could return a TcpConnection object.
    // However self then needs to somehow send incoming packets into that TcpConnection
    // legitimate use case for Arc<Mutex<>> ?
    pub async fn connect(&mut self, dest_ip: Ipv4Addr, dest_port: Port) -> ! {
        todo!()
    }
}

struct TcpSegmentHeader<'a> {
    src_port: Port,
    dst_port: Port,
    sequence_num: u32,
    ack_num: u32,
    /// Actually a u4. Also called HLEN
    data_offset: u8,
    flags: TcpSegmentFlags,
    window_size: u16,
    checksum: u16,
    urgent_pointer: u16,
    /// This is present if the `data_offset` is greater than 5
    /// If it doesn't exist, the slice is just of size 0
    options: &'a [u8],
}

struct TcpSegment<'a> {
    header: TcpSegmentHeader<'a>,
    data: &'a [u8],
}

bitflags! {
    struct TcpSegmentFlags: u8 {
        /// Congestion window reduced
        const CWR = 1;
        /// Function Depends on SYN flag.
        /// If SYN = 1, TCP peer is ECN capable
        /// If SYN = 0, a packet with congestion experienced (ECN=11) in ip header
        const ECE = 1 << 1;
        /// Urgent
        const URG = 1 << 2;
        /// Acknowledgement
        const ACK = 1 << 3;
        /// Push function
        const PSH = 1 << 4;
        /// Reset the connection
        const RST = 1 << 5;
        /// Synchronize the sequence_num
        const SYN = 1 << 6;
        /// Last packet from sender
        const FIN = 1 << 7;
    }
}

impl<'a> TcpSegmentHeader<'a> {
    fn byte_count(&self) -> usize {
        let options_len = self.options.len();

        // this assert is a lil' silly but should catch getting out of sync.
        // It is also very cheap so doesnt really matter
        assert_eq!(
            Self::options_length(self.data_offset),
            options_len,
            "Calculated options length and found options length must match"
        );

        20 + options_len
    }

    fn options_length(data_offset: u8) -> usize {
        if data_offset < 5 {
            0
        } else {
            (data_offset as usize - 5) * 4
        }
    }

    fn to_bytes(&self) -> [u8; 20] {
        let mut out = [0_u8; 20];
        out[0..2].copy_from_slice(&self.src_port.to_be_bytes());
        out[2..4].copy_from_slice(&self.dst_port.to_be_bytes());

        out[4..8].copy_from_slice(&self.sequence_num.to_be_bytes());

        out[8..12].copy_from_slice(&self.ack_num.to_be_bytes());

        out[12] = self.data_offset;
        out[13] = self.flags.bits();
        out[14..16].copy_from_slice(&self.window_size.to_be_bytes());

        out[16..18].copy_from_slice(&self.checksum.to_be_bytes());
        out[18..20].copy_from_slice(&self.urgent_pointer.to_be_bytes());

        out
    }

    fn from_bytes(bytes: &'a [u8]) -> Self {
        let data_offset = bytes[12] & 0xF0;
        Self {
            src_port: u16::from_be_bytes(bytes[0..2].try_into().unwrap()).into(),
            dst_port: u16::from_be_bytes(bytes[2..4].try_into().unwrap()).into(),
            sequence_num: u32::from_be_bytes(bytes[4..8].try_into().unwrap()),
            ack_num: u32::from_be_bytes(bytes[8..12].try_into().unwrap()),
            data_offset,
            flags: TcpSegmentFlags::from_bits_retain(bytes[13]),
            window_size: u16::from_be_bytes(bytes[14..16].try_into().unwrap()),
            checksum: u16::from_be_bytes(bytes[16..18].try_into().unwrap()),
            urgent_pointer: u16::from_be_bytes(bytes[18..20].try_into().unwrap()),
            options: &bytes[20..20 + Self::options_length(data_offset)],
        }
    }
}

impl<'a> TcpSegment<'_> {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.header.byte_count() + self.data.len());
        out.extend_from_slice(&self.header.to_bytes());
        out.extend_from_slice(self.data.as_slice());

        out
    }

    pub fn from_bytes(bytes: &'a [u8]) -> TcpSegment<'a> {
        let header = TcpSegmentHeader::from_bytes(bytes);
        let count = header.byte_count();
        TcpSegment {
            header,
            data: &bytes[count..],
        }
    }
}

#[derive(Debug)]
pub enum TcpError {
    NoInterfaceAvailable,
    ConnectionAlreadyEstablished,
    IpError(IpError),
}

#[derive(Debug, PartialEq, Eq)]
enum TcpConnectionState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    TimeWait,
    LastAck,
    Closing,
}

const TCP_WINDOW_SIZE: u16 = 10;

pub struct TcpConnection<'a> {
    state: TcpConnectionState,
    src_port: Port,
    dest_ip: Ipv4Addr,
    dst_port: Port,
    current_squence_num: u32,
    current_ack_num: u32,

    // this kind of duplicates data since there is also another buffer in the segment handling code
    // but i think this is fine since this is a TCP specific buffer, and the other one is an
    // application level buffer. Plus i'm already not being memory efficient so a little more
    // duplication wont kill me
    sugment_buffer: ArrayQueue<TcpSegment<'a>>,
}

impl<'a> TcpConnection<'a> {
    pub fn new(dest_ip: Ipv4Addr, dst_port: Port, src_port: Port) -> Self {
        TcpConnection {
            state: TcpConnectionState::Closed,
            current_ack_num: 0,
            current_squence_num: 800, // Should be random but this will still probably work
            src_port,
            dest_ip,
            dst_port,
            sugment_buffer: ArrayQueue::new(TCP_WINDOW_SIZE as usize),
        }
    }

    /// Small helper function
    /// Delete me if used in <3 places
    async fn get_interface(&self) -> Result<net::Interface, TcpError> {
        net::get_inferface_for_ip_via_subnet(self.dest_ip)
            .await
            .ok_or(TcpError::NoInterfaceAvailable)
    }

    /// Helper function to create a tcp segment based on the current state
    fn create_base_segment(&self) -> TcpSegment<'a> {
        let header = TcpSegmentHeader {
            src_port: self.src_port,
            dst_port: self.dst_port,
            sequence_num: self.current_squence_num,
            ack_num: self.current_ack_num,
            data_offset: 0,
            flags: TcpSegmentFlags::empty(),
            window_size: TCP_WINDOW_SIZE,
            checksum: 0,
            urgent_pointer: 0,
            options: &[],
        };

        TcpSegment { header, data: &[] }
    }

    pub async fn connect(&mut self) -> Result<(), TcpError> {
        if self.state != TcpConnectionState::Closed {
            return Err(TcpError::ConnectionAlreadyEstablished);
        }
        // Send SYN
        let mut segment = self.create_base_segment();
        segment.header.flags = segment.header.flags.union(TcpSegmentFlags::SYN);
        self.send_segment(segment).await?;
        // Wait for SYN+ACK

        // That means that at this point i need a raw listener to that port, so that i can handle
        // that packet manually. That means that I will have to change the current abstraction.
        // One way to handle it is to have two socket types, a rawSocketHandle, which does
        // OHOH no the socket code has two socket types, where udp is simple and then a TCP socket
        // wraps a tcp connection manager, which then handles all its own internals.
        //  I think somehting like that will work

        // Send ACK

        // Established!!
        Ok(())
    }

    async fn send_segment(&mut self, segment: TcpSegment<'a>) -> Result<(), TcpError> {
        let segment_bytes = segment.to_bytes();

        let source_ip = if self.dest_ip.is_loopback() {
            Ipv4Addr::LOCALHOST
        } else {
            self.get_interface().await?.ip
        };

        let ip_packet = ip::IPv4Packet::from_source_dest_and_data(
            source_ip,
            self.dest_ip,
            ip::IPProtocol::Tcp,
            segment_bytes.as_slice(),
        )
        .map_err(TcpError::IpError)?;

        ip::send_ipv4_packet(ip_packet)
            .await
            .map_err(TcpError::IpError)
    }
}
