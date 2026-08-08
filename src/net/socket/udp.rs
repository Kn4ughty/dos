use alloc::vec::Vec;
use core::{
    net::{Ipv4Addr, SocketAddrV4},
    pin::Pin,
    task::{Context, Poll},
};

use crossbeam_queue::ArrayQueue;
use futures::{Stream, task::AtomicWaker};
use hashbrown::HashMap;
use lazy_static::lazy_static;

use crate::{
    net::ip::{self, IPv4Packet},
    sync::spinlock::Mutex,
};

use super::{Port, SocketError};

const PACKET_QUEUE_LEN: usize = 10;

lazy_static! {
    static ref SOCKET_REGISTRY: Mutex<HashMap<Port, RegistryKey>> = Mutex::new(HashMap::new());
}

struct RegistryKey {
    waker: AtomicWaker,
    incoming_packet_buffer: ArrayQueue<UdpPacketWrapper>,
    binding_address: Ipv4Addr,
}

pub struct UdpPacketWrapper {
    pub src_ip: Ipv4Addr,
    pub dst_ip: Ipv4Addr,
    pub packet: UdpPacket,
}

pub fn handle_incoming_packet(ip_packet: &IPv4Packet<'_>) {
    let Some(udp_packet) = UdpPacket::from_bytes(ip_packet.data) else {
        log::error!("unable to convert ippacket to udp packet. {ip_packet:?}");
        return;
    };

    let dst_port = udp_packet.header.dst_port;

    let registry = SOCKET_REGISTRY.lock();

    let Some(registry_key) = registry.get(&dst_port) else {
        log::debug!("packet did not have slot in da queue");
        return;
    };

    let accept_packet = registry_key.binding_address.is_unspecified()
        || ip_packet.header.destination_address.is_loopback()
        || ip_packet.header.destination_address == registry_key.binding_address;

    if !accept_packet {
        log::debug!(
            "Dropping incoming TCP/UDP packet. Packet header: {:?}",
            udp_packet.header
        );
        return;
    }

    match registry_key.incoming_packet_buffer.push(UdpPacketWrapper {
        src_ip: ip_packet.header.source_address,
        dst_ip: ip_packet.header.destination_address,
        packet: udp_packet,
    }) {
        Ok(()) => {}
        Err(_) => {
            log::error!("socket packet queue full for port: {:?}", dst_port);
        }
    }

    registry_key.waker.wake();
}

pub struct UdpSocket {
    address: SocketAddrV4,
}

impl UdpSocket {
    /// Gives the owner of the socket exclusive access to the port
    /// # Errors
    /// Returns an error of the port is already in use
    pub fn bind(address: SocketAddrV4) -> Result<UdpSocket, SocketError> {
        // put registry entry into the registry
        let mut registry = SOCKET_REGISTRY.lock();

        let key = RegistryKey {
            binding_address: *address.ip(),
            waker: AtomicWaker::new(),
            incoming_packet_buffer: ArrayQueue::new(PACKET_QUEUE_LEN),
        };

        if registry.insert(address.port(), key).is_some() {
            return Err(SocketError::PortAlreadyInUse);
        }

        Ok(UdpSocket { address })
    }

    ///
    /// # Errors
    /// Errors if data is too long or if there is an error turning it into an ip packet or sending
    /// ip packet
    pub async fn send_data(
        &self,
        dst_ip: Ipv4Addr,
        dst_port: Port,
        data: Vec<u8>,
    ) -> Result<(), SocketError> {
        let udp_packet = UdpPacket {
            header: UdpPacketHeader {
                src_port: self.address.port(),
                dst_port,
                length: u16::try_from(data.len() + 8).map_err(|_| SocketError::DataWasTooLong)?,
                checksum: 0,
            },
            data,
        };

        let source_ip = {
            let interface = crate::net::get_inferface_for_ip_via_subnet(dst_ip)
                .await
                .expect("Can get interface");
            interface.ip
        };

        let udp_bytes = udp_packet.to_bytes();
        let ip_packet = ip::IPv4Packet::from_source_dest_and_data(
            source_ip,
            dst_ip,
            ip::IPProtocol::Udp,
            udp_bytes.as_slice(),
        )
        .map_err(SocketError::IpError)?;

        ip::send_ipv4_packet(ip_packet)
            .await
            .map_err(SocketError::IpError)?;

        Ok(())
    }
}

impl Stream for UdpSocket {
    type Item = UdpPacketWrapper;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        {
            let registry = SOCKET_REGISTRY.lock();
            let Some(key) = registry.get(&self.address.port()) else {
                panic!(
                    "No registry entry for udp socket on port: {}!",
                    self.address.port()
                )
            };

            if let Some(packet) = key.incoming_packet_buffer.pop() {
                return Poll::Ready(Some(packet));
            }

            key.waker.register(cx.waker());
        }

        // don't hold lock longer than needed

        {
            let registry = SOCKET_REGISTRY.lock();
            let Some(key) = registry.get(&self.address.port()) else {
                panic!(
                    "No registry entry for udp socket on port: {}!",
                    self.address.port()
                )
            };

            if let Some(packet) = key.incoming_packet_buffer.pop() {
                key.waker.take();
                Poll::Ready(Some(packet))
            } else {
                Poll::Pending
            }
        }
    }
}

impl Drop for UdpSocket {
    fn drop(&mut self) {
        let mut registry = SOCKET_REGISTRY.lock();

        if registry.remove(&self.address.port()).is_none() {
            log::error!(
                "Inconsistent state in socket registry. Port: {:?} was not in registry, and drop was called for udp socket handle",
                self.address.port()
            );
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct UdpPacketHeader {
    pub src_port: Port,
    pub dst_port: Port,
    pub length: u16,
    pub checksum: u16,
}

pub struct UdpPacket {
    pub header: UdpPacketHeader,
    pub data: Vec<u8>,
}

impl UdpPacketHeader {
    #[must_use]
    fn to_bytes(self) -> [u8; 8] {
        let mut out = [0u8; 8];
        out[0..2].copy_from_slice(&self.src_port.to_be_bytes());
        out[2..4].copy_from_slice(&self.dst_port.to_be_bytes());
        out[4..6].copy_from_slice(&self.length.to_be_bytes());
        out[6..8].copy_from_slice(&self.checksum.to_be_bytes());

        out
    }

    #[must_use]
    pub fn from_bytes(bytes: [u8; 8]) -> Self {
        UdpPacketHeader {
            src_port: u16::from_be_bytes(bytes[0..2].try_into().unwrap()),
            dst_port: u16::from_be_bytes(bytes[2..4].try_into().unwrap()),
            length: u16::from_be_bytes(bytes[4..6].try_into().unwrap()),
            checksum: u16::from_be_bytes(bytes[6..8].try_into().unwrap()),
        }
    }
}

impl UdpPacket {
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        // Header is 8 bytes
        let mut joined: Vec<u8> = Vec::with_capacity(8 + self.data.len());
        joined.extend_from_slice(&self.header.to_bytes());
        joined.extend_from_slice(self.data.as_slice());

        // checksum is not calculated because it is optional and easier to just not

        joined
    }

    #[must_use]
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        let mut v = Vec::with_capacity(data.len() - 8);
        v.extend_from_slice(&data[8..]);
        Some(UdpPacket {
            header: UdpPacketHeader::from_bytes(data[0..8].try_into().ok()?),
            data: v,
        })
    }
}
