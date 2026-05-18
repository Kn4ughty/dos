use crate::port::Port;
use crate::spinlock::Mutex;

const CONFIG_ADDRESS: u16 = 0xCF8;
const CONFIG_DATA: u16 = 0xCFC;

/// There must only be one instance of this type.
/// This is because having multiple could lead to terrible bugs, as address and data ports are
/// related.
struct PCIBusDevice {
    address: Port<u32>,
    data: Port<u32>,
}

static PCI_BUS: Mutex<PCIBusDevice> = Mutex::new(PCIBusDevice {
    address: Port::new(CONFIG_ADDRESS),
    data: Port::new(CONFIG_DATA),
});

impl PCIBusDevice {
    pub fn read_dword(&mut self, bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
        let address: u32 = ((bus as u32) << 16)
            | ((slot as u32) << 11)
            | ((func as u32) << 8)
            | (offset & 0xFC) as u32
            | (0x80000000);

        unsafe {
            self.address.write(address);
            self.data.read()
        }
    }

    pub fn read_word(&mut self, bus: u8, slot: u8, func: u8, offset: u8) -> u16 {
        let address: u32 = ((bus as u32) << 16)
            | ((slot as u32) << 11)
            | ((func as u32) << 8)
            | (offset & 0xFC) as u32
            | (0x80000000);

        unsafe {
            self.address.write(address);

            ((self.data.read() >> ((offset & 2) * 8)) & 0xFFFF) as u16
        }
    }
}

pub struct PCIDevice {
    bus: u8,
    slot: u8,
}

impl PCIDevice {
    pub fn new(bus: u8, slot: u8) -> Self {
        PCIDevice { bus, slot }
    }

    pub fn read_word(&mut self, func: u8, offset: u8) -> u16 {
        PCI_BUS.lock().read_word(self.bus, self.slot, func, offset)
    }

    pub fn read_dword(&mut self, func: u8, offset: u8) -> u32 {
        PCI_BUS.lock().read_dword(self.bus, self.slot, func, offset)
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
