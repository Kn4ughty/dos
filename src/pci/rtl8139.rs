use alloc;

const VENDOR_ID: u16 = 0x10EC;
const DEVICE_ID: u16 = 0x8139;

pub struct MACAddress {
    pub mac: [u8; 8],
}

impl core::fmt::Debug for MACAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut s = alloc::string::String::new();
        for (i, seg) in self.mac.iter().enumerate() {
            s += &alloc::format!("{:02x}", seg);
            if i != 7 {
                s += ":";
            }
        }
        f.write_str(s.as_str())
    }
}

#[derive(Debug)]
pub struct RTL8139 {
    base_address: usize,
    mac: MACAddress, // etc
}

// impl core::fmt::Debug for RTL8139 {
//     fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
//         f.debug_struct("RTL8139")
//             .field("base_address", &format_args!("{:#0x}", self.base_address))
//             .field("mac", &format_args!("{:#0x}", self.mac))
//             .finish()
//     }
// }

impl RTL8139 {
    pub fn new(base_address: usize) -> RTL8139 {
        RTL8139 {
            base_address,
            mac: MACAddress {
                mac: unsafe { *(base_address as *const [u8; 8]) },
            },
        }
    }

    pub const fn vendor_id() -> u16 {
        VENDOR_ID
    }

    pub const fn device_id() -> u16 {
        DEVICE_ID
    }
}
