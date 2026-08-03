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
use crate::net::udp::UdpPacketHeader;
use crate::sync::spinlock::Mutex;

/// How many packets get held before being dropped
const SOCKET_RESPONSE_QUEUE_BUFFER_LENGTH: usize = 10;

lazy_static! {
    static ref SOCKET_REGISTRY: Mutex<HashMap<Port, RegistryKey>> = Mutex::new(HashMap::new());
}

struct RegistryKey {
    waker: AtomicWaker,
    packet_buffer: ArrayQueue<SocketResponse>,
}

// pub struct SocketRegistration {
//     waker: AtomicWaker,
//     incoming_queue: ArrayQueue<SocketResponse>,
//     binding_address: Ipv4Addr,
// }

pub fn handle_incoming_packet(packet: &IPv4Packet<'_>) {
    // packket header already validated to be tcp or udp
    // Either way just need to get port and then put into appropriate queue

    let (dst_port, response) = match packet.header.protocol {
        IPProtocol::Tcp => {
            log::debug!("dropping TCP packet");
            return;
        }
        IPProtocol::Udp => {
            let Ok(data) = packet.data[0..8].try_into() else {
                log::warn!("udp packet wasnt even long enough to have a header");
                return;
            };

            let packet_header = UdpPacketHeader::from_bytes(data);

            let mut new_data = Vec::new();
            new_data.extend_from_slice(&packet.data[8..]);
            let sr = SocketResponse {
                source_port: packet_header.src_port,
                data: new_data,
            };
            (packet_header.dst_port, sr)
        }
        _ => {
            panic!("should've been validated beforehannd!!")
        }
    };

    let registry = SOCKET_REGISTRY.lock();
    let Some(registry_key) = registry.get(&dst_port) else {
        log::debug!("packet did not have slot in da queue");
        return;
    };

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
pub enum SocketProtocol {
    Tcp,
    Udp,
}

impl From<SocketProtocol> for IPProtocol {
    fn from(value: SocketProtocol) -> Self {
        match value {
            SocketProtocol::Udp => IPProtocol::Udp,
            SocketProtocol::Tcp => IPProtocol::Tcp,
        }
    }
}

// If the user has a handle, that means they are the effective owner of that port, and all traffic
// to that port should be sent to them via that sockethandle.
// That means that the socket handle should have an awaitable method to get the next response
// I did a similar pattern for the ping response stream code

/// Held by a user to indicate that they have ownership over the send/recv of a specfic port
pub struct SocketHandle {
    port: Port,
    typ: SocketProtocol,
}

impl SocketHandle {
    // RAII is so cool
    // TODO. use binding address
    /// # Errors
    /// Errors if the port is already in use
    pub fn new(port: Port, _binding_address: Ipv4Addr) -> Result<SocketHandle, SocketError> {
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
                },
            );
        }

        Ok(SocketHandle {
            port,
            typ: SocketProtocol::Udp,
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
        let transport_packet = match &self.typ {
            SocketProtocol::Udp => {
                let udp_packet = UdpPacket::new(self.port, dest_port, data);

                udp_packet.to_bytes()
            }
            SocketProtocol::Tcp => {
                todo!("TCP support not yet implemented")
            }
        };

        let interface = super::get_inferface_for_ip_via_subnet(dest_ip)
            .await
            .ok_or(SocketError::UnableToGetNetworkInterface)?;

        let packet = IPv4Packet::from_source_dest_and_data(
            interface.ip,
            dest_ip,
            self.typ.into(),
            transport_packet.as_slice(),
        )
        .map_err(|error| {
            log::error!("Unable to create ipv4packet");
            SocketError::IpError(error)
        })?;

        super::ip::send_ipv4_packet(packet, &interface)
            .await
            .map_err(|e| {
                log::error!("Error sending ip packet");
                SocketError::IpError(e)
            })?;

        Ok(())
    }
}

impl Drop for SocketHandle {
    fn drop(&mut self) {
        {
            let mut registry = SOCKET_REGISTRY.lock();
            registry.remove(&self.port);
        }

        {
            let mut registry = SOCKET_REGISTRY.lock();
            registry.remove(&self.port);
        }
    }
}

pub struct SocketResponse {
    pub source_port: Port,
    pub data: Vec<u8>,
}

impl Stream for SocketHandle {
    type Item = SocketResponse;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut registery = SOCKET_REGISTRY.lock();
        let key = registery
            .get_mut(&self.port)
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

impl Hash for SocketHandle {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        // The port of the socket is the uniquely identifying characteristic
        self.port.hash(state);
    }
}

impl PartialEq for SocketHandle {
    fn eq(&self, other: &Self) -> bool {
        self.port == other.port
    }
}

impl Eq for SocketHandle {}
