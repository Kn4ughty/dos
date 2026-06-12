/// PCI (Peripheral Component Interconnect)
/// See <https://wiki.osdev.org/PCI> for info
use crate::port::Port;
use crate::println;
use crate::spinlock::Mutex;

mod class_codes;
pub use class_codes::ClassCode;

const CONFIG_ADDRESS: u16 = 0xCF8;
const CONFIG_DATA: u16 = 0xCFC;

/// There must only be one instance of this type.
/// This is because having multiple could lead to terrible bugs, as address and data ports are
/// related.
pub struct PCIBusDevice {
    address: Port<u32>,
    data: Port<u32>,
}

pub static PCI_BUS: Mutex<PCIBusDevice> = Mutex::new(PCIBusDevice {
    address: Port::new(CONFIG_ADDRESS),
    data: Port::new(CONFIG_DATA),
});

impl PCIBusDevice {
    pub fn read_dword(&mut self, bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
        #[expect(clippy::cast_lossless, reason = "readability")]
        let address: u32 = ((bus as u32) << 16)
            | ((slot as u32) << 11)
            | ((func as u32) << 8)
            | (offset & 0xFC) as u32
            | (0x8000_0000);

        unsafe {
            self.address.write(address);
            self.data.read()
        }
    }

    pub fn read_word(&mut self, bus: u8, slot: u8, func: u8, offset: u8) -> u16 {
        #[expect(clippy::cast_lossless, reason = "readability")]
        let address: u32 = ((bus as u32) << 16)
            | ((slot as u32) << 11)
            | ((func as u32) << 8)
            | (offset & 0xFC) as u32
            | (0x8000_0000);

        unsafe {
            self.address.write(address);

            ((self.data.read() >> ((offset & 2) * 8)) & 0xFFFF) as u16
        }
    }

    pub fn write_dword(&mut self, bus: u8, slot: u8, func: u8, offset: u8, value: u32) {
        #[expect(clippy::cast_lossless, reason = "readability")]
        let address: u32 = ((bus as u32) << 16)
            | ((slot as u32) << 11)
            | ((func as u32) << 8)
            | (offset & 0xFC) as u32
            | (0x8000_0000);

        unsafe {
            self.address.write(address);
            self.data.write(value);
        }
    }
}

// This doesnt need to be a struct.
// Can be purely proceduar and be fine
pub struct PCIDevice {
    bus: u8,
    slot: u8,
}

impl PCIDevice {
    #[must_use]
    pub fn new(bus: u8, slot: u8) -> Self {
        PCIDevice { bus, slot }
    }

    fn read_word(&mut self, bus: &mut PCIBusDevice, func: u8, offset: u8) -> u16 {
        bus.read_word(self.bus, self.slot, func, offset)
    }

    fn read_dword(&mut self, bus: &mut PCIBusDevice, func: u8, offset: u8) -> u32 {
        bus.read_dword(self.bus, self.slot, func, offset)
    }

    fn write_dword(&mut self, bus: &mut PCIBusDevice, func: u8, offset: u8, value: u32) {
        bus.write_dword(self.bus, self.slot, func, offset, value);
    }

    // This is a function, so that it can be called by itself for later quickly enumerating
    // available pci devices.
    pub fn get_header_type(&mut self, bus: &mut PCIBusDevice) -> u8 {
        let dword = self.read_dword(bus, 0, 0x0C);
        ((dword >> 16) & 0xFF) as u8
    }

    pub fn enable_bus_mastering(&mut self) {
        let bus = &mut PCI_BUS.lock();
        let command = bus.read_word(self.bus, self.slot, 0, 0x4);
        let new_command = command | 0x4;
        let dword = bus.read_dword(self.bus, self.slot, 0, 0x4);
        let new_dword = (dword & 0xFFFF_0000) | (u32::from(new_command) & 0xFFFF);
        self.write_dword(bus, 0, 0x4, new_dword);
    }

    pub fn get_header(&mut self) -> Option<PCIDeviceHeader> {
        let bus: &mut PCIBusDevice = &mut PCI_BUS.lock();

        // Performance Note!
        // It would be signficantly faster to use read_dword, and do bitmath
        // instead of read_word. This is because inb is a very slow call.

        let vendor_id = {
            let t = self.read_word(bus, 0, 0);
            if t == 0xFFFF { None } else { Some(t) }
        }?;

        Some(PCIDeviceHeader {
            vendor_id,
            device_id: self.read_word(bus, 0, 0x2),
            command: self.read_word(bus, 0, 0x4),
            status: self.read_word(bus, 0, 0x6),
            revision_id: (self.read_word(bus, 0, 0x8) & 0xFF) as u8,
            prog_if: ((self.read_word(bus, 0, 0x8) >> 8) & 0xFF) as u8,
            class: {
                let class_code = ((self.read_word(bus, 0, 0xA) >> 8) & 0xFF) as u8;
                let subclass = (self.read_word(bus, 0, 0xa) & 0xFF) as u8;

                ClassCode::try_from((class_code, subclass)).ok()?
            },
            header_type: self.get_header_type(bus),
            base_addr0: self.read_dword(bus, 0, 0x10),
            interrupt_line: (self.read_word(bus, 0, 0x3c) & 0xFF) as u8,
        })
    }
}

#[derive(Debug)]
pub struct PCIDeviceHeader {
    pub vendor_id: u16,
    pub device_id: u16,
    // TODO. Use bitflags!
    pub command: u16,
    pub status: u16,
    pub revision_id: u8,
    pub prog_if: u8,
    pub class: class_codes::ClassCode,
    pub header_type: u8,
    /// Pointer to mmio address
    pub base_addr0: u32,
    pub interrupt_line: u8,
}

pub fn lspci() {
    for bus in 0..=255 {
        for device in 0..=31 {
            let mut pci_device = crate::pci::PCIDevice::new(bus, device);
            if let Some(header) = pci_device.get_header() {
                println!("{:?}", header);
            }
        }
    }
}
