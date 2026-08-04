use alloc::vec::Vec;
use core::hash::Hash;
use core::net::Ipv4Addr;
use core::pin::Pin;
use core::task::{Context, Poll};
use crossbeam_queue::ArrayQueue;

use futures::Stream;
use futures::task::AtomicWaker;
use hashbrown::HashMap;
use lazy_static::lazy_static;

use super::IPv4Packet;
use super::udp::UdpPacket;

use crate::net::ip;
use crate::net::ip::IPProtocol;
use crate::net::tcp;
use crate::net::udp;
use crate::sync::spinlock::Mutex;

/// How many packets get held before being dropped
const SOCKET_RESPONSE_QUEUE_BUFFER_LENGTH: usize = 10;

lazy_static! {
    static ref SOCKET_REGISTRY: Mutex<HashMap<Port, RegistryKey>> = Mutex::new(HashMap::new());
}

struct RegistryKey {
    waker: AtomicWaker,
    packet_buffer: ArrayQueue<SocketResponse>,
    binding_address: Ipv4Addr,
}

pub fn handle_incoming_packet(packet: &IPv4Packet<'_>) {
    let (dst_port, response) = match packet.header.protocol {
        IPProtocol::Tcp => {
            log::debug!("dropping TCP packet");
            return;
        }
        IPProtocol::Udp => {
            let Ok(header_bytes) = packet.data[0..8].try_into() else {
                log::warn!("UDP packet wasnt long enough to have a header, dropping");
                return;
            };

            let packet_header = udp::UdpPacketHeader::from_bytes(header_bytes);

            let mut new_data = Vec::new();
            new_data.extend_from_slice(&packet.data[8..]);
            let sr = SocketResponse {
                source_port: packet_header.src_port,
                data: new_data,
            };
            (packet_header.dst_port, sr)
        }
        _ => {
            // TODO. Work out how to encode that in types
            unreachable!("Packet type should have been validated to be tcp or UDP before this")
        }
    };

    let registry = SOCKET_REGISTRY.lock();
    let Some(registry_key) = registry.get(&dst_port) else {
        log::debug!("packet did not have slot in da queue");
        return;
    };

    let accept_packet = registry_key.binding_address.is_unspecified()
        || packet.header.destination_address.is_loopback()
        || packet.header.destination_address == registry_key.binding_address;

    if !accept_packet {
        log::debug!(
            "Dropping incoming TCP/UDP packet. Packet header: {:?}",
            packet.header
        );
        return;
    }

    match registry_key.packet_buffer.push(response) {
        Ok(()) => {}
        Err(_) => {
            log::error!("socket packet queue full for port: {:?}", dst_port);
        }
    }

    registry_key.waker.wake();
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Port(pub u16);

impl From<u16> for Port {
    fn from(value: u16) -> Self {
        Port(value)
    }
}

#[derive(Debug)]
pub enum SocketError {
    PortAlreadyInUse,
    UnableToGetNetworkInterface,
    IpError(ip::IpError),
}

#[derive(Debug, Clone, Copy)]
pub enum SocketProtocolType {
    Tcp,
    Udp,
}

impl From<SocketProtocolType> for IPProtocol {
    fn from(value: SocketProtocolType) -> Self {
        match value {
            SocketProtocolType::Udp => IPProtocol::Udp,
            SocketProtocolType::Tcp => IPProtocol::Tcp,
        }
    }
}

impl<'a> From<&mut SocketHandle<'a>> for IPProtocol {
    fn from(value: &mut SocketHandle<'a>) -> Self {
        match value {
            SocketHandle::Tcp(_) => IPProtocol::Tcp,
            SocketHandle::Udp(_) => IPProtocol::Udp,
        }
    }
}

/// Held by a user to indicate that they have ownership over the send/recv of a specific port
pub enum SocketHandle<'a> {
    /// UDP just contains a port because it is connectionless
    Udp(Port),
    Tcp(tcp::TcpSocket<'a>),
}

impl<'a> SocketHandle<'a> {
    // RAII is so cool
    /// Creates a new socket handle
    /// It is currently not possible to have a port be both a TCP and UDP connection at once. Oh
    /// well.
    /// # Errors
    /// Errors if the port is already in use
    ///
    pub fn new(
        port: Port,
        binding_address: Ipv4Addr,
        typ: SocketProtocolType,
    ) -> Result<SocketHandle<'a>, SocketError> {
        {
            let mut registry = SOCKET_REGISTRY.lock();

            if registry.contains_key(&port) {
                return Err(SocketError::PortAlreadyInUse);
            }

            registry.insert(
                port,
                RegistryKey {
                    waker: AtomicWaker::new(),
                    packet_buffer: ArrayQueue::new(SOCKET_RESPONSE_QUEUE_BUFFER_LENGTH),
                    binding_address,
                },
            );
        }

        Ok(match typ {
            SocketProtocolType::Udp => SocketHandle::Udp(port),
            #[expect(unused)]
            SocketProtocolType::Tcp => SocketHandle::Tcp(todo!()),
        })
    }

    // We have ownership over this port,
    /// # Errors
    /// Errors if unable to create or send the ipv4 packet
    pub async fn send_data(
        &mut self,
        dest_ip: Ipv4Addr,
        dest_port: Port,
        data: &[u8],
    ) -> Result<(), SocketError> {
        let transport_packet = match self {
            SocketHandle::Udp(port) => {
                let udp_packet = UdpPacket::new(*port, dest_port, data);

                udp_packet.to_bytes()
            }
            SocketHandle::Tcp(_tcp_socket) => {
                todo!("TCP support not yet implemented")
            }
        };

        let interface = super::get_inferface_for_ip_via_subnet(dest_ip)
            .await
            .ok_or(SocketError::UnableToGetNetworkInterface)?;

        let packet = IPv4Packet::from_source_dest_and_data(
            interface.ip,
            dest_ip,
            self.into(),
            transport_packet.as_slice(),
        )
        .map_err(|error| {
            log::error!("Unable to create ipv4packet");
            SocketError::IpError(error)
        })?;

        super::ip::send_ipv4_packet(packet).await.map_err(|e| {
            log::error!("Error sending ip packet");
            SocketError::IpError(e)
        })?;

        Ok(())
    }

    #[must_use]
    pub fn get_port(&self) -> Port {
        match self {
            SocketHandle::Tcp(tcp_socket) => tcp_socket.port,
            SocketHandle::Udp(port) => *port,
        }
    }
}

impl Drop for SocketHandle<'_> {
    fn drop(&mut self) {
        let port = self.get_port();

        {
            let mut registry = SOCKET_REGISTRY.lock();
            registry.remove(&port);
        }

        {
            let mut registry = SOCKET_REGISTRY.lock();
            registry.remove(&port);
        }
    }
}

pub struct SocketResponse {
    pub source_port: Port,
    pub data: Vec<u8>,
}

impl Stream for SocketHandle<'_> {
    type Item = SocketResponse;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut registery = SOCKET_REGISTRY.lock();
        let key = registery
            .get_mut(&self.get_port())
            .expect("impossible to have handle on closed port");

        key.waker.register(cx.waker());

        // omg woke up. That means there is a packet available in the queue

        let response = key.packet_buffer.pop();

        if let Some(response) = response {
            key.waker.take();
            Poll::Ready(Some(response))
        } else {
            Poll::Pending
        }
    }
}
