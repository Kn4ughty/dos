use core::hash::Hash;
use core::net::Ipv4Addr;

use hashbrown::HashSet;
use lazy_static::lazy_static;

use crate::sync::spinlock::Mutex;

// acts as a sparse list of ports
lazy_static! {
    // using spinlock here may cause problems with preemptive multitasking but is fine for now
    static ref SOCKET_REGISTRY: Mutex<HashSet<Socket>> = Mutex::new(HashSet::new());
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Port(u16);

impl From<u16> for Port {
    fn from(value: u16) -> Self {
        Port(value)
    }
}

#[derive(Debug)]
pub enum SocketError {
    PortAlreadyInUse,
}

/// A socket identifier
#[derive(Clone, Copy)]
pub struct Socket {
    port: Port,
    bound_address: Ipv4Addr,
}

impl Socket {
    // RAII
    pub fn new(port: Port, binding_address: Ipv4Addr) -> Result<SocketHandle, SocketError> {
        let mut registry = SOCKET_REGISTRY.lock();
        let socket_ident = Socket {
            port,
            bound_address: binding_address,
        };

        if registry.contains(&socket_ident) {
            return Err(SocketError::PortAlreadyInUse);
        }

        registry.insert(socket_ident);

        Ok(SocketHandle {
            socket: socket_ident,
        })
    }
}

// If the user has a handle, that means they are the effective owner of that port, and all traffic
// to that port should be sent to them via that sockethandle.
// That means that the socket handle should have an awaitable method to get the next response
// I did a similar pattern for the ping response stream code
pub struct SocketHandle {
    socket: Socket,
}

impl Hash for Socket {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        // The port of the socket is the uniquely identifying characteristic and only one service
        // must be bound to a port
        self.port.hash(state);
    }
}

impl PartialEq for Socket {
    fn eq(&self, other: &Self) -> bool {
        self.port == other.port
    }
}

impl Eq for Socket {}
