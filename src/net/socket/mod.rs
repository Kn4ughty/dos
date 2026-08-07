use super::IPv4Packet;

use crate::net::ip::IPProtocol;

pub mod udp;

pub type Port = u16;

pub fn handle_incoming_packet(packet: &IPv4Packet<'_>) {
    match packet.header.protocol {
        IPProtocol::Tcp => {
            log::debug!("dropping TCP packet");
        }
        IPProtocol::Udp => {
            udp::handle_incoming_packet(packet);
        }
        _ => {
            // TODO. Work out how to encode that in types
            unreachable!("Packet type should have been validated to be tcp or UDP before this")
        }
    }
}
