use alloc::{vec, vec::Vec};
use conquer_once::spin::OnceCell;
use core::task::{Context, Poll};
use crossbeam_queue::ArrayQueue;
use futures_util::{Stream, StreamExt, task::AtomicWaker};
use x86_64::instructions::interrupts::without_interrupts;

use crate::{
    net::{ethernet::EtherType, nic::rtl8139::RTL},
    println,
};

mod arp;
mod ethernet;
mod ip;
mod nic;

use ethernet::EthernetPacket;

const IP: u32 = const { u32::from_be_bytes([192, 168, 10, 2]) };

const PACKET_QUEUE_SIZE: usize = 4;
static PACKET_QUEUE: OnceCell<ArrayQueue<Vec<u8>>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

// static EthernetDevice: OnceCell<RTL8139> = OnceCell::uninit();

pub fn test_packet() {
    let mac = RTL.get().unwrap().lock().get_mac();
    let arp = arp::ArpPacket::new_arp_request(mac, IP, u32::from_be_bytes([192, 168, 10, 1]));
    let ep = EthernetPacket {
        destination: ethernet::BROADCAST_MAC,
        typ: EtherType::Arp,
        source: mac,
        data: &arp.to_bytes(),
    };
    let mut buf = vec![0u8; ep.total_len()];
    ep.write_into(&mut buf.as_mut_slice());
    without_interrupts(|| RTL.get().unwrap().lock().send_packet(&buf.as_mut_slice()))
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

    loop {
        if let Some(packet) = nns.next().await
            && let Ok(ep) = EthernetPacket::try_from(packet.as_slice())
        {
            if ep.typ == EtherType::Arp {}
            // println!("received ep: {:?}", ep);
        }
    }
}
