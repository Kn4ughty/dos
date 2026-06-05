use alloc::vec::Vec;
use conquer_once::spin::OnceCell;
use core::task::{Context, Poll};
use crossbeam_queue::ArrayQueue;
use futures_util::{Stream, StreamExt, task::AtomicWaker};

use crate::println;

mod ethernet;
mod nic;

use ethernet::EthernetPacket;

const PACKET_QUEUE_SIZE: usize = 4;
static PACKET_QUEUE: OnceCell<ArrayQueue<Vec<u8>>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

// static EthernetDevice: OnceCell<RTL8139> = OnceCell::uninit();

pub fn init() {
    PACKET_QUEUE
        .try_init_once(|| ArrayQueue::new(PACKET_QUEUE_SIZE))
        .expect("packet queue already init");
    nic::rtl8139::find_rtl();
}

pub fn send_arp() {
    // ARP packet: who has 10.0.2.2? tell 10.0.2.15
    // (QEMU's default gateway is 10.0.2.2, guest is 10.0.2.15)
    let mut packet = [0u8; 42];

    // Ethernet header
    packet[0..6].copy_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]); // dst: broadcast

    let mac = nic::rtl8139::RTL.get().unwrap().lock().get_mac();

    packet[6..12].copy_from_slice(&mac); // src: our mac
    packet[12..14].copy_from_slice(&[0x08, 0x06]); // ethertype: ARP

    // ARP body
    packet[14..16].copy_from_slice(&[0x00, 0x01]); // hardware type: ethernet
    packet[16..18].copy_from_slice(&[0x08, 0x00]); // protocol type: IPv4
    packet[18] = 6; // hardware size
    packet[19] = 4; // protocol size
    packet[20..22].copy_from_slice(&[0x00, 0x01]); // opcode: request
    packet[22..28].copy_from_slice(&mac); // sender mac
    packet[28..32].copy_from_slice(&[10, 0, 2, 15]); // sender IP: 10.0.2.15
    packet[32..38].copy_from_slice(&[0, 0, 0, 0, 0, 0]); // target mac: unknown
    packet[38..42].copy_from_slice(&[10, 0, 2, 2]); // target IP: 10.0.2.2 (QEMU gateway)

    // self.send_packet(&packet);
    nic::rtl8139::RTL.get().unwrap().lock().send_packet(&packet);
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
        if let Some(packet) = nns.next().await {
            let ep = EthernetPacket::try_from(packet.as_slice());
            println!("received ep: {:?}", ep);
        }
    }
}
