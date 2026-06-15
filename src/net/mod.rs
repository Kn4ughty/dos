use alloc::vec::Vec;
use conquer_once::spin::OnceCell;
use core::{
    net::Ipv4Addr,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll},
};
use crossbeam_queue::ArrayQueue;
use futures_util::{Stream, StreamExt, task::AtomicWaker};
use log::{debug, error, trace, warn};
use x86_64::instructions::interrupts::without_interrupts;

use crate::net::{ethernet::EtherType, nic::rtl8139::RTL};

mod arp;
mod ethernet;
mod icmp;
mod ip;
mod nic;

use ethernet::{EthernetFrame, EthernetPacket};

const PACKET_QUEUE_SIZE: usize = 16;
static PACKET_QUEUE: OnceCell<ArrayQueue<Vec<u8>>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

static TX_WAKER: AtomicWaker = AtomicWaker::new();
static TX_COMPLETE: AtomicBool = AtomicBool::new(false);

pub fn notify_tx_complete() {
    TX_COMPLETE.store(true, Ordering::Release);
    TX_WAKER.wake();
}

pub async fn send_frame(interface: Interface, frame: EthernetFrame) {
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

#[derive(Clone, Copy)]
pub enum WhichInterface {
    RTL8139,
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

#[derive(Clone, Copy)]
pub struct Interface {
    mac: ethernet::MacAddress,
    ip: Ipv4Addr,
    gateway: Ipv4Addr,
    subnet_mask: Ipv4Addr,
    which: WhichInterface,
}

trait EthernetDevice {
    fn send_packet(&mut self, frame: &EthernetFrame);
    fn receive_packet(&mut self) -> Option<Vec<u8>>;
}

pub fn init() {
    log::debug!("Network init");
    PACKET_QUEUE
        .try_init_once(|| ArrayQueue::new(PACKET_QUEUE_SIZE))
        .expect("packet queue already init");
    nic::rtl8139::find_rtl();
}

pub fn push_packet(packet: Vec<u8>) {
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

pub struct NetworkStream {}

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
pub async fn get_packet() {
    let mut nns = NetworkStream {};

    let intf = Interface {
        mac: RTL.get().unwrap().lock().get_mac(),
        ip: Ipv4Addr::from_octets([192, 168, 10, 2]),
        gateway: Ipv4Addr::from_octets([192, 168, 10, 1]),
        subnet_mask: Ipv4Addr::from_octets([0xFF, 0xFF, 0xFF, 0x00]),
        which: WhichInterface::RTL8139,
    };

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
                    arp::handle_arp_incoming(&a, &intf).await;
                }
            }
            EtherType::IPv4 => {
                if let Ok(ip_packet) = ip::IPv4Packet::try_from(ep.data) {
                    // Snoop it. We know that this MAC owns this IP, so we can update for free
                    arp::ARP_TABLE
                        .lock()
                        .insert(ip_packet.header.source_address, ep.source);

                    ip::handle_packet(&ip_packet, intf).await;
                }
            }
        }
    }
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
