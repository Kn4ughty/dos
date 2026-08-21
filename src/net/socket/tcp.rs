use alloc::vec;
use alloc::vec::Vec;
use core::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    pin::Pin,
    range::Range,
    task::{Context, Poll},
};
use crossbeam_queue::ArrayQueue;
use futures::{Stream, StreamExt, task::AtomicWaker};

use bitflags::bitflags;
use hashbrown::HashMap;
use lazy_static::lazy_static;
use socket::Port;

use crate::net::{
    self,
    ip::{self, IPv4Packet, IpError},
    socket,
};
use crate::sync::spinlock::Mutex;

const TCP_WINDOW_SIZE: u16 = 10;

const MAX_PENDING_CONNECTIONS: usize = 10;

lazy_static! {
    /// From local port, (listener) to registry key
    static ref REGISTRY: Mutex<HashMap<Port, RegistryKey>> = Mutex::new(HashMap::new());
}

// TODO. Implement drop for the socket types to auto close connections

// This will need waker registered to it so that a method to poll for new connections can be
// implemented
pub struct RegistryKey {
    binding_address: Ipv4Addr,

    /// Connections that have not been accepted / handed out as a `TcpStream` yet
    pending_connections: ArrayQueue<TcpConnection>,

    /// From remote address to a queue
    connections: HashMap<SocketAddrV4, TcpConnection>,

    waker: AtomicWaker,
}

// this function has no protection against a connection opening flooding attack.
// I will allocate all the required resources and then run out of memory :p
pub fn handle_incoming_packet(packet: &IPv4Packet<'_>) {
    let segment = match TcpSegment::from_bytes(packet.data) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Unable to turn ip packet into TcpSegment. Error: {e:?}");
            return;
        }
    };

    let mut registry = REGISTRY.lock();
    // Check that we have an open port for the incoming packet
    let Some(key) = registry.get_mut(&segment.header.dst_port) else {
        log::debug!(
            "No open port ({}) for incoming tcp packet",
            segment.header.dst_port
        );
        return;
    };

    // check binding address
    if !socket::should_accept_packet(packet.header.destination_address, key.binding_address) {
        log::debug!(
            "Dropping TCP packet because did not match binding address. dest: {}, bind: {}",
            packet.header.destination_address,
            key.binding_address
        );
        return;
    }

    let remote_addr = SocketAddrV4::new(packet.header.source_address, segment.header.src_port);
    let local_addr = SocketAddrV4::new(packet.header.destination_address, segment.header.dst_port);

    // Put packet into the correct socket queue
    match key.connections.entry(remote_addr) {
        hashbrown::hash_map::Entry::Occupied(o) => {
            if let Err(e) = o.into_mut().recieve_segment(packet) {
                log::error!("tcp error: {e:?}");
            }
        }
        hashbrown::hash_map::Entry::Vacant(_v) => {
            // This is the first incoming connection from this source

            // need to handle acking the connection in this case
            let mut new_connection = TcpConnection::new(remote_addr, local_addr);
            new_connection.recieve_segment(packet);
            key.pending_connections.push(new_connection);
            key.waker.wake();
        }
    }
}

/// Listens for incoming TCP connections
/// This is the user accessible version of a registry entry
/// there needs to be two since the registry handles actually taking in packets
pub struct TcpListener {
    binding_address: SocketAddrV4,
}

impl<'a> TcpListener {
    #[must_use]
    pub fn bind(binding_address: SocketAddrV4) -> Result<TcpListener, TcpError> {
        let out = TcpListener { binding_address };

        // Setup registry entry
        let mut reg = REGISTRY.lock();
        match reg.entry(binding_address.port()) {
            hashbrown::hash_map::Entry::Vacant(v) => {
                v.insert(RegistryKey {
                    binding_address: *binding_address.ip(),
                    pending_connections: ArrayQueue::new(MAX_PENDING_CONNECTIONS),
                    connections: HashMap::new(),
                    waker: AtomicWaker::new(),
                });
                Ok(out)
            }
            hashbrown::hash_map::Entry::Occupied(_o) => Err(TcpError::PortAlreadyInUse),
        }
    }

    /// Waits for an incoming connection, and then returns a handle to that connection
    pub async fn accept(&mut self) -> TcpStream {
        self.next()
            .await
            .expect("nuh uh cant be error cause i say so")
    }

    fn try_get_stream(binding_address: SocketAddrV4) -> Option<TcpStream> {
        let mut reg = REGISTRY.lock();
        let entry = reg
            .get_mut(&binding_address.port())
            .expect("RegistryValue removed while TCPListener still exists is invalid");

        if let Some(connection) = entry.pending_connections.pop() {
            let remote_addr = connection.remote_addr;
            let local_addr = connection.local_addr;

            // Convert into accepted connection
            if let Some(conn) = entry.connections.insert(remote_addr, connection) {
                panic!(
                    "there was already a connection for this address even though it was in the pending queue. This indicates a logic error in the new connection handling code. conn: {:?}",
                    conn
                )
            }
            // then convert into Stream
            let stream = TcpStream {
                local_addr,
                remote_addr,
            };
            return Some(stream);
        }
        None
    }
}

impl Stream for TcpListener {
    type Item = TcpStream;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        log::trace!("poll for tcp listener called");

        if let Some(stream) = Self::try_get_stream(self.binding_address) {
            return Poll::Ready(Some(stream));
        }

        // There was no connection in the pending queue. register waker and wait
        {
            let mut reg = REGISTRY.lock();
            let entry = reg
                .get_mut(&self.binding_address.port())
                .expect("RegistryValue removed while TcpListener still exists is invalid");

            entry.waker.register(cx.waker());
        }

        log::trace!("omg tcp listener woke up i think");

        if let Some(stream) = Self::try_get_stream(self.binding_address) {
            log::debug!("TcpListneer found connection on second go around");
            Poll::Ready(Some(stream))
        } else {
            log::debug!("Did not find conneciton in the queue");
            Poll::Pending
        }
    }
}

/// User accessible access to a TCP connection with a remote host
#[derive(Debug)]
pub struct TcpStream {
    local_addr: SocketAddrV4,
    remote_addr: SocketAddrV4,
}

impl TcpStream {
    pub async fn connect(target: SocketAddrV4) -> Result<TcpStream, TcpError> {
        todo!()
        // Create a new tcp connection object into the registry

        // setup the connection
        // make sure it is connected

        // done
    }

    /// Writes into the buffer and returns how many bytes were read
    /// # Errors
    /// Returns an error if uh idk work this out.
    /// connection closed by remote?
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, TcpError> {
        log::debug!("read called for stream. self: {self:?}");
        let mut registry = REGISTRY.lock();
        let reg_key = registry
            .get_mut(&self.local_addr.port())
            .expect("port should exist for outgoing connection");

        let connection = reg_key
            .connections
            .get_mut(&self.remote_addr)
            .ok_or(TcpError::ConnectionDoesntExist)?;

        Ok(connection.read(buf))
    }
}

#[derive(Debug)]
struct TcpConnection {
    local_addr: SocketAddrV4,
    remote_addr: SocketAddrV4,
    state: TcpConnectionState,
    current_squence_num: u32,
    /// most recent packet with have sent an ack for
    current_ack_num: u32,
    last_read_byte: u32,
    segment_buffer: [Option<TcpSegment>; TCP_WINDOW_SIZE as usize],
}

impl TcpConnection {
    // /// The source port is picked randomly from the (<https://en.wikipedia.org/wiki/Ephemeral_port>)[Ephemeral port] range
    pub fn new(remote_addr: SocketAddrV4, local_addr: SocketAddrV4) -> Self {
        let random_seq_num = 0; // Should be random but this will work
        TcpConnection {
            local_addr,
            remote_addr,
            state: TcpConnectionState::Closed,
            current_ack_num: 0,
            current_squence_num: random_seq_num,
            last_read_byte: random_seq_num,
            segment_buffer: [const { None }; TCP_WINDOW_SIZE as usize],
        }
    }

    pub fn recieve_segment(&mut self, ip_packet: &IPv4Packet) -> Result<(), TcpError> {
        let Some(slot) = self.segment_buffer.iter_mut().find(|se| se.is_none()) else {
            log::error!("TCP segment buffer is full! this shouldnt really happen :(");
            return Err(TcpError::TcpSegmentBufferFull);
        };
        *slot = Some(TcpSegment::from_bytes(ip_packet.data)?);

        // TODO. Check if need to ack the packet and stuff

        Ok(())
    }

    /// Small helper function
    /// Delete me if used in <3 places
    async fn get_interface(&self) -> Result<net::Interface, TcpError> {
        net::get_inferface_for_ip_via_subnet(*self.remote_addr.ip())
            .await
            .ok_or(TcpError::NoInterfaceAvailable)
    }

    /// Helper function to create a tcp segment based on the current state
    fn create_base_segment(&self) -> TcpSegment {
        let header = TcpSegmentHeader {
            src_port: self.local_addr.port(),
            dst_port: self.remote_addr.port(),
            sequence_num: self.current_squence_num,
            ack_num: self.current_ack_num,
            data_offset: 0,
            flags: TcpSegmentFlags::empty(),
            window_size: TCP_WINDOW_SIZE,
            checksum: 0,
            urgent_pointer: 0,
            options: vec![],
        };

        TcpSegment {
            header,
            data: vec![],
        }
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

        // Send ACK

        // Established!!
        Ok(())
    }

    async fn send_segment(&mut self, segment: TcpSegment) -> Result<(), TcpError> {
        let segment_bytes = segment.to_bytes();

        let source_ip = if self.remote_addr.ip().is_loopback() {
            Ipv4Addr::LOCALHOST
        } else {
            self.get_interface().await?.ip
        };

        let ip_packet = ip::IPv4Packet::from_source_dest_and_data(
            source_ip,
            *self.remote_addr.ip(),
            ip::IPProtocol::Tcp,
            segment_bytes.as_slice(),
        )
        .map_err(TcpError::IpError)?;

        ip::send_ipv4_packet(ip_packet)
            .await
            .map_err(TcpError::IpError)
    }

    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        // goal. Find the range of data starting from the last_read_byte to at most the current_ack_num
        // Can be done in 2 loops. One to find the right segment sequence,
        // and second loop to put that data into the buffer.

        // let mut segments: Vec<&TcpSegment> = Vec::new();

        let mut found_range = Range::from(self.last_read_byte..self.last_read_byte);

        log::trace!("Entering read range finding loop");
        loop {
            let mut should_break = true;
            for segment in &self.segment_buffer {
                let Some(segment) = segment else { continue };

                let start_byte = segment.header.sequence_num;
                let end_byte = start_byte
                    + u32::try_from(segment.data.len())
                        .expect("I do not handle tramissions bigger than 4gb");

                // This segment extends the found range, or is in the middle of extending the range
                if found_range.end == start_byte
                    || (start_byte < found_range.start && end_byte > found_range.start)
                {
                    found_range.end = end_byte;

                    // updated range this loop so keep looping
                    should_break = false;
                }
            }

            if should_break {
                break;
            }
        }

        log::trace!("Updatable range into the buf is {:?}", found_range);

        // Now need to go back into the segments, and write into buf, if the segment is within the
        // range, and the buf can fit it.
        todo!();

        0
    }
}

#[derive(Debug)]
struct TcpSegmentHeader {
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
    options: Vec<u8>,
}

#[derive(Debug)]
struct TcpSegment {
    header: TcpSegmentHeader,
    data: Vec<u8>,
}

bitflags! {
    #[derive(Debug)]
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

impl TcpSegmentHeader {
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

    fn from_bytes(bytes: &[u8]) -> Result<Self, TcpError> {
        if bytes.len() < 20 {
            log::trace!("other len was not enought");
            return Err(TcpError::HigherLevelPacketWasTooShort);
        }

        let data_offset = (bytes[12] & 0xF0) >> 4;

        if bytes.len() < 20 + Self::options_length(data_offset) {
            log::trace!(
                "options len was not enough. data_offset = {}, optlen = {}",
                data_offset,
                Self::options_length(data_offset)
            );
            return Err(TcpError::HigherLevelPacketWasTooShort);
        }

        let mut options_vec = Vec::new();
        options_vec.extend_from_slice(&bytes[20..20 + Self::options_length(data_offset)]);

        Ok(Self {
            src_port: u16::from_be_bytes(bytes[0..2].try_into().unwrap()),
            dst_port: u16::from_be_bytes(bytes[2..4].try_into().unwrap()),
            sequence_num: u32::from_be_bytes(bytes[4..8].try_into().unwrap()),
            ack_num: u32::from_be_bytes(bytes[8..12].try_into().unwrap()),
            data_offset,
            flags: TcpSegmentFlags::from_bits_retain(bytes[13]),
            window_size: u16::from_be_bytes(bytes[14..16].try_into().unwrap()),
            checksum: u16::from_be_bytes(bytes[16..18].try_into().unwrap()),
            urgent_pointer: u16::from_be_bytes(bytes[18..20].try_into().unwrap()),
            options: options_vec,
        })
    }
}

impl TcpSegment {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.header.byte_count() + self.data.len());
        out.extend_from_slice(&self.header.to_bytes());
        out.extend_from_slice(self.data.as_slice());

        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<TcpSegment, TcpError> {
        let header = TcpSegmentHeader::from_bytes(bytes)?;
        let count = header.byte_count();

        let mut dv = Vec::new();
        dv.extend_from_slice(&bytes[count..]);

        Ok(TcpSegment { header, data: dv })
    }
}

impl PartialEq for TcpSegment {
    fn eq(&self, other: &Self) -> bool {
        // i am skeptical of this
        self.header.sequence_num == other.header.sequence_num
    }
}

impl Eq for TcpSegment {}

impl PartialOrd for TcpSegment {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TcpSegment {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.header.sequence_num.cmp(&other.header.sequence_num)
    }
}

#[derive(Debug)]
pub enum TcpError {
    NoInterfaceAvailable,
    ConnectionAlreadyEstablished,
    ConnectionDoesntExist,
    TcpSegmentBufferFull,
    HigherLevelPacketWasTooShort,
    PortAlreadyInUse,
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

static LAST_EPHEMERAL_PORT_REGISTERED: Mutex<EphemeralPortTracker> =
    Mutex::new(EphemeralPortTracker::new());

struct EphemeralPortTracker {
    range: Range<u16>,
    last_registered: u16,
}

impl EphemeralPortTracker {
    const fn new() -> Self {
        let range = Range {
            start: 49152_u16,
            end: u16::MAX,
        };
        EphemeralPortTracker {
            range,
            last_registered: range.start,
        }
    }

    fn get_new() -> Port {
        let lepr = LAST_EPHEMERAL_PORT_REGISTERED.lock();
        let mut new = lepr.last_registered + 1;

        if new >= lepr.range.end {
            new = lepr.range.start;
        }

        new
    }
}
