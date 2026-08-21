use alloc::{string::String, vec, vec::Vec};
use core::{
    net::{Ipv4Addr, SocketAddrV4},
    str::FromStr,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll},
    time::Duration,
};
use crossbeam_queue::ArrayQueue;
use futures::future::{Either, select};
use futures_util::{Stream, StreamExt, task::AtomicWaker};
use lazy_static::lazy_static;
use log::{debug, trace, warn};
use no_std_async::RwLock;
use x86_64::instructions::interrupts::without_interrupts;

use crate::{
    net::{ethernet::EtherType, ip::IPv4Packet},
    println,
    sync::spinlock::Mutex,
    task::{block_on, sleep::sleep_duration},
    time,
};

mod arp;
mod ethernet;
mod icmp;
mod ip;
mod nic;
pub mod socket;

use ethernet::{EthernetFrame, EthernetPacket};

static WAKER: AtomicWaker = AtomicWaker::new();

const PACKET_QUEUE_SIZE: usize = 16;
lazy_static! {
    static ref PACKET_QUEUE: Mutex<ArrayQueue<Vec<u8>>> =
        Mutex::new(ArrayQueue::new(PACKET_QUEUE_SIZE));
    static ref INTERFACES: RwLock<Vec<Interface>> = RwLock::new(Vec::new());
}

async fn get_inferface_for_ip_via_subnet(destination: Ipv4Addr) -> Option<Interface> {
    let interfaces = INTERFACES.read().await;
    interfaces
        .iter()
        .find(|i| i.is_same_subnet(destination))
        .or_else(|| interfaces.first())
        .copied() // This wil cause TOCTOU bugs when changing device settings at runtime is implemented
}

async fn is_ip_for_us(ip: Ipv4Addr) -> bool {
    let interfaces = INTERFACES.read().await;
    ip.is_loopback() || interfaces.iter().any(|i| i.ip == ip)
}

/// Contains data about a network connection, but does not actually hold the nic
/// TODO. Fix socket implementation so interface can be private
#[derive(Clone, Copy)]
pub struct Interface {
    mac: ethernet::MacAddress,
    ip: Ipv4Addr,
    gateway: Ipv4Addr,
    subnet_mask: Ipv4Addr,
    which: WhichInterface,
}

impl Interface {
    #[must_use]
    pub fn is_same_subnet(&self, ip: Ipv4Addr) -> bool {
        self.subnet_mask & self.gateway == self.subnet_mask & ip
    }
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

    let mut target = block_on(INTERFACES.write());
    target.push(intf);
}

static TX_WAKER: AtomicWaker = AtomicWaker::new();
static TX_COMPLETE: AtomicBool = AtomicBool::new(false);

/// Called by network cards to notify that packet transmission has been completed
fn notify_tx_complete() {
    TX_COMPLETE.store(true, Ordering::Release);
    TX_WAKER.wake();
}

async fn send_frame(interface: &Interface, frame: EthernetFrame, is_loopback: bool) {
    TX_COMPLETE.store(false, Ordering::Release);

    if is_loopback {
        push_packet(frame.as_bytes().to_vec());
        return;
    }
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
    let queue = PACKET_QUEUE.lock();

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

        let queue = PACKET_QUEUE.lock();

        if let Some(packet) = queue.pop() {
            return Poll::Ready(Some(packet));
        }
        WAKER.register(cx.waker());

        if let Some(packet) = queue.pop() {
            WAKER.take();
            log::trace!("Networkstream woken and packet was returned");
            Poll::Ready(Some(packet))
        } else {
            log::trace!("Network stream woken but no packet in queue");
            Poll::Pending
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
                    arp::handle_arp_incoming(&a).await;
                }
            }
            EtherType::IPv4 => {
                if let Ok(ip_packet) = ip::IPv4Packet::try_from(ep.data) {
                    // Snoop it. We know that this MAC owns this IP, so we can update for free
                    arp::ARP_TABLE
                        .lock()
                        .insert(ip_packet.header.source_address, ep.source);

                    ip::handle_incoming_packet(&ip_packet).await;
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

    icmp::ping::ping(address, 5).await;

    println!("ping elapsed: {:?}", start.elapsed());
}

pub async fn ncu(args: &[&str]) {
    if args.len() < 2 {
        println!("too few args");
        return;
    }

    let destination = match Ipv4Addr::from_str(args[0]) {
        Ok(a) => a,
        Err(e) => {
            println!("Could not turn arg to address: {e:?}");
            return;
        }
    };

    let dst_port = match u16::from_str(args[1]) {
        Ok(p) => p,
        Err(e) => {
            println!("Invalid port: {e:?}");
            return;
        }
    };

    let Some(interface) = get_inferface_for_ip_via_subnet(destination).await else {
        log::error!("No interface found");
        return;
    };

    // TODO. Generate random num for source port
    let Ok(mut handle) = socket::udp::UdpSocket::bind(SocketAddrV4::new(interface.ip, 12_456))
    else {
        log::error!("Could not obtain handle to port");
        return;
    };

    let Ok(()) = handle.send_data(destination, dst_port, vec![]).await else {
        log::error!("could not send data");
        return;
    };

    log::info!("sent data");

    let response = select(handle.next(), sleep_duration(Duration::from_secs(5))).await;

    match response {
        Either::Left((response, _)) => {
            let response = response.unwrap();
            let s = String::from_utf8_lossy_owned(response.packet.data);
            println!("{:?}", s);
        }
        Either::Right(_) => {
            println!("timeout");
        }
    }
}

/// TCP listen on specified port
pub async fn ncl(args: &[&str]) {
    #[expect(clippy::len_zero)]
    if args.len() < 1 {
        println!("Not enough arguments. Expected >= 1");
        return;
    }

    let Ok(port) = u16::from_str(args[0]) else {
        println!("Could not turn port to string");
        return;
    };

    let mut listener =
        match socket::tcp::TcpListener::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port)) {
            Ok(l) => l,
            Err(e) => {
                println!("Error binding port: {e:?}");
                return;
            }
        };

    let mut stream = listener.accept().await;
    stream.ensure_established().await;
    let mut buf = [0_u8; 20];
    println!("{:?}", stream.read(&mut buf));
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
