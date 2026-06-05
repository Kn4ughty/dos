#[derive(Debug)]
pub enum EthernetError {
    MACWrongLengthSlice,
    EthernetPacketNotLongEnough,
}

pub struct MacAddress([u8; 6]);

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

#[derive(Debug)]
pub struct EthernetPacket<'a> {
    destination: MacAddress,
    source: MacAddress,
    typ: u16,
    data: &'a [u8],
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
            typ: u16::from_be_bytes(v[12..14].try_into().unwrap()),
            data: &v[14..v.len()],
        })
    }
}
