use crate::{
    net::Interface,
    println,
    tryfrom::{tryfrom, tryfrom2arg},
};

pub fn handle_icmp(packet: ICMPPacket, interface: Interface) {
    println!("handling icmp: {:?}", packet);
    if packet.typ == ControlMessageType::EchoRequest(NoSubcode::NoCode) {
        println!("omg reply");
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

pub enum ICMPError {
    UnknownControlMessageType,
    PacketNotLongEnough,
}

tryfrom2arg! {
    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq)]
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
    #[derive(Debug, PartialEq, Eq)]
    pub enum NoSubcode {
        NoCode = 0,
    }, u8
}

tryfrom! {
    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq)]
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
    #[derive(Debug, PartialEq, Eq)]
    pub enum RedirectSubcode {
        /// Redirect for the network (or subnet)
        RedirectForNetwork         = 0,
        /// Redirect for the host
        RedirectForHost            = 1,
        /// Redirect for the type of service and network
        RedirectForTosAndNetwork   = 2,
        /// Redirect for the type of service and host
        RedirectForTosAndHost      = 3,
    }, u8
}

tryfrom! {
    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq)]
    pub enum RouterAdvertisementSubcode {
        /// RFC 3344
        NormalRouterAdvertisement  = 0,
        /// RFC 3344 — router does not route common traffic
        DoesNotRouteCommonTraffic  = 16,
    }, u8
}

tryfrom! {
    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq)]
    pub enum TimeExceededSubcode {
        /// TTL expired in transit — this is what traceroute exploits
        TtlExceededInTransit        = 0,
        /// Fragment reassembly time exceeded
        FragmentReassemblyExceeded  = 1,
    }, u8
}

tryfrom! {
    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq)]
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
    #[derive(Debug, PartialEq, Eq)]
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
    #[derive(Debug, PartialEq, Eq)]
    pub enum ExtendedEchoReplySubcode {
        /// RFC 8335
        NoError                    = 0,
        MalformedQuery             = 1,
        NoSuchInterface            = 2,
        NoSuchTableEntry           = 3,
        MultipleInterfacesSatisfy  = 4,
    }, u8
}
