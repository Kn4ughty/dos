use alloc::vec::Vec;
use conquer_once::spin::OnceCell;
use core::{
    net::Ipv4Addr,
    str::FromStr,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll},
};
use crossbeam_queue::ArrayQueue;
use futures_util::{Stream, StreamExt, task::AtomicWaker};
use log::{debug, error, trace, warn};
use no_std_async::RwLock;
use x86_64::instructions::interrupts::without_interrupts;

use crate::{net::ethernet::EtherType, println, task::block_on, time};

mod arp;
mod ethernet;
mod icmp;
mod ip;
mod nic;

use ethernet::{EthernetFrame, EthernetPacket};

const PACKET_QUEUE_SIZE: usize = 16;
static PACKET_QUEUE: OnceCell<ArrayQueue<Vec<u8>>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

// The interface needs to be mutable. It could be a onceCell, but adding nic
// is adding interfaces at runtime something i need? mmm yeah cause stuff can be unplugged.
// that only applies to usb devices though.
// The interface needs to allow multiple reads at once.
// That means that a RwLock needs to be used.
// Using a mutex for now
static INTERFACE: RwLock<Option<Interface>> = RwLock::new(None);

/// Contains data about a network connection, but does not actually hold the nic
#[derive(Clone, Copy)]
struct Interface {
    mac: ethernet::MacAddress,
    ip: Ipv4Addr,
    gateway: Ipv4Addr,
    subnet_mask: Ipv4Addr,
    which: WhichInterface,
}

#[derive(Clone, Copy)]
enum WhichInterface {
    RTL8139,
}

trait EthernetDevice {
    fn send_packet(&mut self, frame: &EthernetFrame);
    fn receive_packet(&mut self) -> Option<Vec<u8>>;
}

impl WhichInterface {
    fn with_device<F, R>(self, f: F) -> R
    where
        F: FnOnce(&mut dyn EthernetDevice) -> R,
    {
        match self {
            WhichInterface::RTL8139 => {
                let mut guard = nic::rtl8139::RTL.get().unwrap().lock();
                f(&mut *guard)
            }
        }
    }
}

pub fn init() {
    log::debug!("Network init");
    PACKET_QUEUE
        .try_init_once(|| ArrayQueue::new(PACKET_QUEUE_SIZE))
        .expect("packet queue already init");
    nic::rtl8139::find_rtl();

    let intf = Interface {
        mac: nic::rtl8139::RTL
            .get()
            .expect("RTL8139 device shoulde exist")
            .lock()
            .get_mac(),
        ip: Ipv4Addr::from_octets([192, 168, 10, 2]),
        gateway: Ipv4Addr::from_octets([192, 168, 10, 1]),
        subnet_mask: Ipv4Addr::from_octets([0xFF, 0xFF, 0xFF, 0x00]),
        which: WhichInterface::RTL8139,
    };

    let mut target = block_on(INTERFACE.write());
    *target = Some(intf);
}

static TX_WAKER: AtomicWaker = AtomicWaker::new();
static TX_COMPLETE: AtomicBool = AtomicBool::new(false);

/// Called by network cards to notify that packet transmission has been completed
fn notify_tx_complete() {
    TX_COMPLETE.store(true, Ordering::Release);
    TX_WAKER.wake();
}

async fn send_frame(interface: &Interface, frame: EthernetFrame) {
    TX_COMPLETE.store(false, Ordering::Release);
    without_interrupts(|| interface.which.with_device(|dev| dev.send_packet(&frame)));

    futures_util::future::poll_fn(|cx| {
        if TX_COMPLETE.load(Ordering::Acquire) {
            Poll::Ready(())
        } else {
            TX_WAKER.register(cx.waker());
            if TX_COMPLETE.load(Ordering::Acquire) {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }
    })
    .await;
}

fn push_packet(packet: Vec<u8>) {
    let Ok(queue) = PACKET_QUEUE.try_get() else {
        error!("Packet queue not initialised. Dropping packet");
        return;
    };

    if queue.push(packet).is_ok() {
        WAKER.wake();
    } else {
        warn!("Packet queue fulL! Dropping packet");
    }
}

struct NetworkStream {}

impl Stream for NetworkStream {
    type Item = Vec<u8>;

    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        trace!("Poll for NetworkStream called");

        let queue = PACKET_QUEUE
            .try_get()
            .expect("packet queue not initialised!");

        if let Some(packet) = queue.pop() {
            return Poll::Ready(Some(packet));
        }
        WAKER.register(cx.waker());

        match queue.pop() {
            Some(packet) => {
                WAKER.take();
                Poll::Ready(Some(packet))
            }
            None => Poll::Pending,
        }
    }
}

/// Init must be called before this
pub async fn loop_networking() {
    let mut nns = NetworkStream {};

    loop {
        let Some(packet) = nns.next().await else {
            continue;
        };

        let Ok(ep) = EthernetPacket::try_from(packet.as_slice()) else {
            debug!(
                "ep error! {:?}",
                EthernetPacket::try_from(packet.as_slice())
            );
            let t = u16::from_be_bytes(packet.as_slice()[12..14].try_into().unwrap());
            debug!("ep type from error {:#0x?}", t);
            continue;
        };

        match ep.typ {
            EtherType::Arp => {
                if let Ok(a) = arp::ArpPacket::try_from(ep.data) {
                    arp::handle_arp_incoming(&a, &INTERFACE.read().await.unwrap()).await;
                }
            }
            EtherType::IPv4 => {
                if let Ok(ip_packet) = ip::IPv4Packet::try_from(ep.data) {
                    // Snoop it. We know that this MAC owns this IP, so we can update for free
                    arp::ARP_TABLE
                        .lock()
                        .insert(ip_packet.header.source_address, ep.source);

                    ip::handle_packet(&ip_packet, &INTERFACE.read().await.unwrap()).await;
                }
            }
        }
    }
}

pub async fn ping(args: &[&str]) {
    let start = time::Instant::now();

    let address = match Ipv4Addr::from_str(args[0]) {
        Ok(a) => a,
        Err(e) => {
            println!("Could not turn arg to address: {e:?}");
            return;
        }
    };

    icmp::ping::ping_once(address).await;

    println!("ping elapsed: {:?}", start.elapsed());
}

fn ones_complement_checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;

    let mut chunks = data.chunks_exact(2);
    for chunk in chunks.by_ref() {
        sum += u32::from(u16::from_be_bytes([chunk[0], chunk[1]]));
    }

    if let Some(&leftover) = chunks.remainder().first() {
        sum += u32::from(u16::from_be_bytes([leftover, 0]));
    }

    // fold cary bits
    while sum >> 16 != 0 {
        sum = (sum & 0xFF_FF) + (sum >> 16);
    }

    #[expect(clippy::cast_possible_truncation)]
    !(sum as u16)
}
