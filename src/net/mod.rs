use alloc::vec::Vec;
use conquer_once::spin::OnceCell;
use core::task::{Context, Poll};
use crossbeam_queue::ArrayQueue;
use futures_util::{Stream, StreamExt, task::AtomicWaker};

use crate::println;

mod intel_e1000e;
pub mod rtl8139;

const PACKET_QUEUE_SIZE: usize = 4;
static PACKET_QUEUE: OnceCell<ArrayQueue<Vec<u8>>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

pub fn init() {
    PACKET_QUEUE
        .try_init_once(|| ArrayQueue::new(PACKET_QUEUE_SIZE))
        .expect("packet queue already init");
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
            println!("omg gt some: {:?}", packet);
        }
    }
}
