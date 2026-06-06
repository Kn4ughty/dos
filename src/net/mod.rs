use alloc::vec::Vec;
use conquer_once::spin::OnceCell;
use core::{
    net::Ipv4Addr,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll},
};
use crossbeam_queue::ArrayQueue;
use futures_util::{Stream, StreamExt, task::AtomicWaker};
use x86_64::instructions::interrupts::without_interrupts;

use crate::{
    net::{ethernet::EtherType, nic::rtl8139::RTL},
    println,
};

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

/// Config for a single network interface
#[derive(Clone, Copy)]
pub struct InterfaceConfig {
    mac: ethernet::MacAddress,
    ip: Ipv4Addr,
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
    config: InterfaceConfig,
    which: WhichInterface,
}

trait EthernetDevice {
    fn send_packet(&mut self, frame: &EthernetFrame);
    fn receive_packet(&mut self) -> Option<Vec<u8>>;
}

pub fn init() {
    PACKET_QUEUE
        .try_init_once(|| ArrayQueue::new(PACKET_QUEUE_SIZE))
        .expect("packet queue already init");
    nic::rtl8139::find_rtl();
}

pub fn push_packet(packet: Vec<u8>) {
    let Ok(queue) = PACKET_QUEUE.try_get() else {
        println!("Packet queue not made. Dropping packet");
        return;
    };

    if queue.push(packet).is_ok() {
        WAKER.wake();
    } else {
        println!("Packet queue fulL! Dropping packet");
    }
}

pub struct NetworkStream {}

impl Stream for NetworkStream {
    type Item = Vec<u8>;

    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        println!("Getting packet");

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

    let mac = RTL.get().unwrap().lock().get_mac();
    let intf = Interface {
        config: InterfaceConfig {
            mac,
            ip: Ipv4Addr::from_octets([192, 168, 10, 2]),
        },
        which: WhichInterface::RTL8139,
    };

    loop {
        if let Some(packet) = nns.next().await
            && let Ok(ep) = EthernetPacket::try_from(packet.as_slice())
        {
            match ep.typ {
                EtherType::Arp => {
                    if let Ok(a) = arp::ArpPacket::try_from(ep.data) {
                        arp::handle_arp(&a, &intf);
                    }
                }
                EtherType::IPv4 => {
                    if let Ok(ip_packet) = ip::IPv4Packet::try_from(ep.data) {
                        // println!("{:?}", ip_packet);
                        ip::handle_packet(&ip_packet, intf).await;
                    }
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
