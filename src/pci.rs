use crate::port;
use crate::port::Port;

const CONFIG_ADDRESS: u16 = 0xCF8;
const CONFIG_DATA: u16 = 0xCFC;

pub struct PCIDevice {
    bus: u8,
    slot: u8,
    config_address: Port<u32>,
    config_data: Port<u32>,
}

impl PCIDevice {
    pub fn new(bus: u8, slot: u8) -> Self {
        let out_port = port::Port::<u32>::new(CONFIG_ADDRESS);
        let in_port = port::Port::<u32>::new(CONFIG_DATA);
        PCIDevice {
            bus,
            slot,
            config_address: out_port,
            config_data: in_port,
        }
    }

    pub fn read_word(&mut self, func: u8, offset: u8) -> u16 {
        let lbus: u32 = self.bus as u32;
        let lslot: u32 = self.slot as u32;
        let lfunc: u32 = func as u32;

        let address: u32 =
            (lbus << 16) | (lslot << 11) | (lfunc << 8) | (offset & 0xFC) as u32 | (0x80000000);

        unsafe { self.config_address.write(address) };

        let tmp = (unsafe { self.config_data.read() } >> ((offset & 2) * 8)) & 0xFFFF;
        tmp as u16
    }

    pub fn read_dword(&mut self, func: u8, offset: u8) -> u32 {
        let address: u32 = ((self.bus as u32) << 16)
            | ((self.slot as u32) << 11)
            | ((func as u32) << 8)
            | (offset & 0xFC) as u32
            | (0x80000000);

        unsafe {
            self.config_address.write(address);
            self.config_data.read()
        }
    }

    pub fn check_vendor(&mut self) -> u16 {
        let vendor = self.read_word(0, 0);
        if vendor != 0xFFFF { vendor } else { 0 }
    }

    pub fn get_class_code(&mut self) -> u8 {
        self.read_word(0, 0x4) as u8
    }

    pub fn get_header_type(&mut self) -> u8 {
        let dword = self.read_dword(0, 0x0C);
        ((dword >> 16) & 0xFF) as u8
    }
}

pub fn pci_config_read_word(bus: u8, slot: u8, func: u8, offset: u8) -> u16 {
    let lbus: u32 = bus as u32;
    let lslot: u32 = slot as u32;
    let lfunc: u32 = func as u32;

    let address: u32 =
        (lbus << 16) | (lslot << 11) | (lfunc << 8) | (offset & 0xFC) as u32 | (0x80000000);

    let mut out_port = port::Port::<u32>::new(0xcf8);
    unsafe { out_port.write(address) };

    let mut in_port = port::Port::<u32>::new(0xCFC);

    let tmp = (unsafe { in_port.read() } >> ((offset & 2) * 7)) & 0xFFFF;
    tmp as u16
}
