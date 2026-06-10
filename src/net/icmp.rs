use alloc::{vec, vec::Vec};
use log::debug;

use crate::{
    net::{
        Interface, arp,
        ethernet::{self, EthernetPacket},
        ip::IPv4Packet,
        ones_complement_checksum,
    },
    tryfrom::{TryFrom2argAndReverse, tryfrom},
};

pub async fn handle_icmp(packet: &IPv4Packet<'_>, interface: Interface) {
    // Packet already validated to have us as its destination
    let Ok(icmpp) = ICMPPacket::try_from(packet.data) else {
        return;
    };

    debug!("handling icmp: {:?}", packet);
    if icmpp.typ == ControlMessageType::EchoRequest(NoSubcode::NoCode) {
        let mut dest_mac = arp::ARP_TABLE
            .lock()
            .get(&packet.header.source_address)
            .copied();

        if dest_mac.is_none() {
            log::debug!(
                "No ARP entry for icmp request {}, attempting to do a lookup",
                packet.header.source_address
            );
            dest_mac = super::arp::find_target(packet.header.source_address, &interface).await;
        }

        let Some(dest_mac) = dest_mac else { return };

        let mut response = ICMPPacket {
            typ: ControlMessageType::EchoReply(NoSubcode::NoCode),
            checksum: 0,
            other: icmpp.other,
            data: icmpp.data,
        };
        response.calc_new_checksum();

        let resp_bytes = response.to_bytes();
        let ipv4 = IPv4Packet::from_source_dest_and_data(
            interface.config.ip,
            packet.header.source_address,
            resp_bytes.as_slice(),
        );

        debug!("Sending icmp response: {:?}", ipv4);

        let ip_bytes = ipv4.expect("Packet constructed incorrectly").to_bytes();
        let ep = EthernetPacket {
            destination: dest_mac,
            source: interface.config.mac,
            typ: ethernet::EtherType::IPv4,
            data: ip_bytes.as_slice(),
        };

        super::send_frame(interface, ep.into()).await;
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ICMPPacket<'a> {
    typ: ControlMessageType,
    checksum: u16,
    other: u32,
    data: &'a [u8],
}

impl<'a> TryFrom<&'a [u8]> for ICMPPacket<'a> {
    type Error = ICMPError;
    fn try_from(v: &'a [u8]) -> Result<Self, Self::Error> {
        if v.len() < 8 {
            return Err(ICMPError::PacketNotLongEnough);
        }

        Ok(ICMPPacket {
            typ: ControlMessageType::try_from((v[0], v[1]))
                .map_err(|_| ICMPError::UnknownControlMessageType)?,
            checksum: u16::from_be_bytes(v[2..4].try_into().unwrap()),
            other: u32::from_be_bytes(v[4..8].try_into().unwrap()),
            data: &v[8..v.len()],
        })
    }
}

impl ICMPPacket<'_> {
    /// Replaces the checksum of self with a correct one
    pub fn calc_new_checksum(&mut self) {
        self.checksum = 0;
        let bytes = self.to_bytes();
        self.checksum = ones_complement_checksum(bytes.as_slice());

        debug_assert_eq!(
            ones_complement_checksum(self.to_bytes().as_slice()),
            0,
            "checksum with self should be 0"
        );
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = vec![0u8; self.total_len()];
        buf[0] = self.typ.outer_as();
        buf[1] = self.typ.inner_as();
        buf[2..=3].copy_from_slice(&self.checksum.to_be_bytes());
        buf[4..8].copy_from_slice(&self.other.to_be_bytes());
        buf[8..self.total_len()].copy_from_slice(self.data);

        buf
    }

    fn total_len(&self) -> usize {
        8 + self.data.len()
    }
}

#[derive(Debug)]
pub enum ICMPError {
    UnknownControlMessageType,
    PacketNotLongEnough,
}

#[derive(Debug)]
#[expect(unused)]
pub struct EchoPacket {
    identifier: u16,
    sequence_num: u16,
}

impl From<u32> for EchoPacket {
    fn from(v: u32) -> Self {
        EchoPacket {
            identifier: ((v >> 16) & 0xFFFF) as u16,
            sequence_num: (v & 0xFFFF) as u16,
        }
    }
}

TryFrom2argAndReverse! {
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ControlMessageType {
        EchoReply(NoSubcode) = 0,
        DestinationUnreachable(DestinationUnreachableSubcode) = 3,
        /// Deprecated (RFC 6633)
        SourceQuench(NoSubcode) = 4,
        Redirect(RedirectSubcode) = 5,
        EchoRequest(NoSubcode) = 8,
        RouterAdvertisement(RouterAdvertisementSubcode) = 9,
        RouterSolicitation(NoSubcode) = 10,
        TimeExceeded(TimeExceededSubcode) = 11,
        ParameterProblem(ParameterProblemSubcode) = 12,
        /// Deprecated (RFC 6918)
        Timestamp(NoSubcode) = 13,
        /// Deprecated (RFC 6918)
        TimestampReply(NoSubcode) = 14,
        /// Deprecated (RFC 6918)
        InformationRequest(NoSubcode) = 15,
        /// Deprecated (RFC 6918)
        InformationReply(NoSubcode) = 16,
        /// Deprecated (RFC 6918)
        AddressMaskRequest(NoSubcode) = 17,
        /// Deprecated (RFC 6918)
        AddressMaskReply(NoSubcode) = 18,
        Photuris(PhototurisSubcode) = 40,
        ExtendedEchoRequest(NoSubcode) = 42,
        ExtendedEchoReply(ExtendedEchoReplySubcode) = 43,
    }, u8
}

// Used for message types that have no meaningful subcodes (code is always 0)
tryfrom! {
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum NoSubcode {
        NoCode = 0,
    }, u8
}

tryfrom! {
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum DestinationUnreachableSubcode {
        /// RFC 792
        NetUnreachable                          = 0,
        /// RFC 792
        HostUnreachable                         = 1,
        /// RFC 792
        ProtocolUnreachable                     = 2,
        /// RFC 792
        PortUnreachable                         = 3,
        /// RFC 792 — packet required fragmentation but DF bit was set
        FragmentationNeededDfSet                = 4,
        /// RFC 792
        SourceRouteFailed                       = 5,
        /// RFC 1122
        DestinationNetworkUnknown               = 6,
        /// RFC 1122
        DestinationHostUnknown                  = 7,
        /// RFC 1122
        SourceHostIsolated                      = 8,
        /// RFC 1122
        NetworkAdministrativelyProhibited       = 9,
        /// RFC 1122
        HostAdministrativelyProhibited          = 10,
        /// RFC 1122
        NetworkUnreachableForTos                = 11,
        /// RFC 1122
        HostUnreachableForTos                   = 12,
        /// RFC 1812
        CommunicationAdministrativelyProhibited = 13,
        /// RFC 1812
        HostPrecedenceViolation                 = 14,
        /// RFC 1812
        PrecedenceCutoffInEffect                = 15,
    }, u8
}

tryfrom! {
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum RedirectSubcode {
        /// Redirect for the network (or subnet)
        Network         = 0,
        /// Redirect for the host
        Host            = 1,
        /// Redirect for the type of service and network
        TosAndNetwork   = 2,
        /// Redirect for the type of service and host
        TosAndHost      = 3,
    }, u8
}

tryfrom! {
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum RouterAdvertisementSubcode {
        /// RFC 3344
        NormalRouterAdvertisement  = 0,
        /// RFC 3344 — router does not route common traffic
        DoesNotRouteCommonTraffic  = 16,
    }, u8
}

tryfrom! {
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum TimeExceededSubcode {
        /// TTL expired in transit — this is what traceroute exploits
        TtlExceededInTransit        = 0,
        /// Fragment reassembly time exceeded
        FragmentReassemblyExceeded  = 1,
    }, u8
}

tryfrom! {
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ParameterProblemSubcode {
        /// Pointer field indicates the octet where the error was detected
        PointerIndicatesError       = 0,
        /// RFC 1108
        MissingRequiredOption       = 1,
        BadLength                   = 2,
    }, u8
}

tryfrom! {
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum PhototurisSubcode {
        /// RFC 2521
        BadSpi                 = 0,
        AuthenticationFailed   = 1,
        DecompressionFailed    = 2,
        DecryptionFailed       = 3,
        NeedAuthentication     = 4,
        NeedAuthorization      = 5,
    }, u8
}

tryfrom! {
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ExtendedEchoReplySubcode {
        /// RFC 8335
        NoError                    = 0,
        MalformedQuery             = 1,
        NoSuchInterface            = 2,
        NoSuchTableEntry           = 3,
        MultipleInterfacesSatisfy  = 4,
    }, u8
}
