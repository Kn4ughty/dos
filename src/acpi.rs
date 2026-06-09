use core::ptr::NonNull;

use acpi::{AcpiTables, rsdp, sdt::fadt::Fadt};
use conquer_once::spin::OnceCell;
use x86_64::PhysAddr;

pub static FADT: OnceCell<Fadt> = OnceCell::uninit();

pub fn init() {
    let Ok(rsdp) = (unsafe { rsdp::Rsdp::search_for_on_bios(MyHandler) }) else {
        log::error!("failed to find bios acpi");
        return;
    };

    let Ok(table) =
        (unsafe { AcpiTables::from_rsdt(MyHandler, rsdp.revision(), rsdp.rsdt_address as usize) })
    else {
        log::error!("Failed to get AcpiTables");
        return;
    };

    let fadt = *table
        .find_table::<acpi::sdt::fadt::Fadt>()
        .expect("FADT must exist (because i say so)");
    FADT.try_init_once(|| fadt)
        .expect("acpi::init() must be called only once");
}

#[derive(Debug, Clone, Copy)]
struct MyHandler;

impl acpi::Handler for MyHandler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> acpi::PhysicalMapping<Self, T> {
        let phys = PhysAddr::new(physical_address as u64);
        let virt = crate::mem::phys_to_virt(phys);
        let ptr = NonNull::new(virt.as_mut_ptr()).unwrap();

        acpi::PhysicalMapping {
            physical_start: physical_address,
            virtual_start: ptr,
            region_length: size,
            mapped_length: size,
            handler: MyHandler,
        }
    }

    fn unmap_physical_region<T>(_region: &acpi::PhysicalMapping<Self, T>) {}

    // These are physical addresses
    fn read_u8(&self, address: usize) -> u8 {
        read_addr(address)
    }

    fn read_u16(&self, address: usize) -> u16 {
        read_addr(address)
    }

    fn read_u32(&self, address: usize) -> u32 {
        read_addr(address)
    }

    fn read_u64(&self, address: usize) -> u64 {
        read_addr(address)
    }

    fn write_u8(&self, _address: usize, _value: u8) {
        unimplemented!()
    }

    fn write_u16(&self, _address: usize, _value: u16) {
        unimplemented!()
    }

    fn write_u32(&self, _address: usize, _value: u32) {
        unimplemented!()
    }

    fn write_u64(&self, _address: usize, _value: u64) {
        unimplemented!()
    }

    fn read_io_u8(&self, _port: u16) -> u8 {
        unimplemented!()
    }

    fn read_io_u16(&self, _port: u16) -> u16 {
        unimplemented!()
    }

    fn read_io_u32(&self, _port: u16) -> u32 {
        unimplemented!()
    }

    fn write_io_u8(&self, _port: u16, _value: u8) {
        unimplemented!()
    }

    fn write_io_u16(&self, _port: u16, _value: u16) {
        unimplemented!()
    }

    fn write_io_u32(&self, _port: u16, _value: u32) {
        unimplemented!()
    }

    fn read_pci_u8(&self, _address: acpi::PciAddress, _offset: u16) -> u8 {
        unimplemented!()
    }

    fn read_pci_u16(&self, _address: acpi::PciAddress, _offset: u16) -> u16 {
        unimplemented!()
    }

    fn read_pci_u32(&self, _address: acpi::PciAddress, _offset: u16) -> u32 {
        unimplemented!()
    }

    fn write_pci_u8(&self, _address: acpi::PciAddress, _offset: u16, _value: u8) {
        unimplemented!()
    }

    fn write_pci_u16(&self, _address: acpi::PciAddress, _offset: u16, _value: u16) {
        unimplemented!()
    }

    fn write_pci_u32(&self, _address: acpi::PciAddress, _offset: u16, _value: u32) {
        unimplemented!()
    }

    fn nanos_since_boot(&self) -> u64 {
        unimplemented!()
    }

    fn stall(&self, _microseconds: u64) {
        unimplemented!()
    }

    fn sleep(&self, _milliseconds: u64) {
        unimplemented!()
    }

    fn create_mutex(&self) -> acpi::Handle {
        unimplemented!()
    }

    fn acquire(&self, _mutex: acpi::Handle, _timeout: u16) -> Result<(), acpi::aml::AmlError> {
        unimplemented!()
    }

    fn release(&self, _mutex: acpi::Handle) {
        unimplemented!()
    }
}

fn read_addr<T>(addr: usize) -> T
where
    T: Copy,
{
    let virt = crate::mem::phys_to_virt(PhysAddr::new(addr as u64));
    unsafe { *virt.as_ptr() }
}
