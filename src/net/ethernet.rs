use crate::tryfrom::tryfrom;

#[derive(Debug)]
pub enum EthernetError {
    MACWrongLengthSlice,
    EthernetPacketNotLongEnough,
    UnknownEtherType,
}

#[derive(Clone, Copy)]
pub struct MacAddress(pub [u8; 6]);

pub const BROADCAST_MAC: MacAddress = const { MacAddress([0xff; 6]) };

impl core::fmt::Debug for MacAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for (i, seg) in &mut self.0.iter().enumerate() {
            write!(f, "{:02X}", seg)?;
            if i != 5 {
                write!(f, ":")?;
            }
        }
        Ok(())
    }
}

impl TryFrom<&[u8]> for MacAddress {
    type Error = EthernetError;

    fn try_from(v: &[u8]) -> Result<Self, Self::Error> {
        Ok(MacAddress(
            v.try_into()
                .map_err(|_| EthernetError::MACWrongLengthSlice)?,
        ))
    }
}

impl From<[u8; 6]> for MacAddress {
    fn from(value: [u8; 6]) -> Self {
        MacAddress(value)
    }
}

#[derive(Debug)]
pub struct EthernetPacket<'a> {
    pub destination: MacAddress,
    pub source: MacAddress,
    pub typ: EtherType,
    pub data: &'a [u8],
}

impl<'a> TryFrom<&'a [u8]> for EthernetPacket<'a> {
    type Error = EthernetError;

    fn try_from(v: &'a [u8]) -> Result<Self, Self::Error> {
        if v.len() < 14 {
            return Err(EthernetError::EthernetPacketNotLongEnough);
        }

        Ok(EthernetPacket {
            destination: MacAddress::try_from(&v[0..6]).unwrap(),
            source: MacAddress::try_from(&v[6..12]).unwrap(),
            typ: EtherType::try_from(u16::from_be_bytes(v[12..14].try_into().unwrap()))
                .map_err(|_| EthernetError::UnknownEtherType)?,
            data: &v[14..v.len()],
        })
    }
}

impl EthernetPacket<'_> {
    pub fn write_into(&self, buf: &mut [u8]) {
        buf[0..6].copy_from_slice(&self.destination.0);
        buf[6..12].copy_from_slice(&self.source.0);
        buf[12..14].copy_from_slice(&(self.typ as u16).to_be_bytes());
        buf[14..14 + self.data.len()].copy_from_slice(self.data);
    }

    pub fn total_len(&self) -> usize {
        14 + self.data.len()
    }
}

tryfrom! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[repr(u16)]
    #[non_exhaustive]
    pub enum EtherType {
        IPv4 = 0x0800,
        Arp = 0x0806,
    }, u16
}
