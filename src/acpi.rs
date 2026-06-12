use core::{
    ptr::NonNull,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use acpi::{AcpiTables, Handler, aml::Interpreter, rsdp, sdt::fadt::Fadt};
use conquer_once::spin::OnceCell;
use x86_64::PhysAddr;

use crate::spinlock::Mutex;

use crate::time;

static INTERPRETER: OnceCell<Mutex<Interpreter<MyHandler>>> = OnceCell::uninit();

pub static FADT: OnceCell<Fadt> = OnceCell::uninit();

pub fn init() {
    log::debug!("ACPI init");
    let handler = MyHandler;

    // root system descriptor pointer
    let Ok(rsdp) = (unsafe { rsdp::Rsdp::search_for_on_bios(handler) }) else {
        log::error!("failed to find bios acpi");
        return;
    };

    let Ok(table) =
        (unsafe { AcpiTables::from_rsdt(handler, rsdp.revision(), rsdp.rsdt_address as usize) })
    else {
        log::error!("Failed to get AcpiTables");
        return;
    };

    // fixed ACPI desciprion table
    let fadt_mapping = table.find_table::<acpi::sdt::fadt::Fadt>().unwrap();
    let fadt = fadt_mapping.get();

    FADT.try_init_once(|| *fadt)
        .expect("acpi::init() must be called only once");

    // The FADT contains the DSDT

    // The DSDT (Differentiated System Description Table) is an executable program written in
    // bytecode in a language called AML.
    // Then this program describes all the system stuff, thermal zones, power button, connected
    // devices etc.

    let dsdt = table.dsdt().expect("DSDT must exist");

    let revision = dsdt.revision;

    let header_size = core::mem::size_of::<acpi::sdt::SdtHeader>();
    let aml_len = dsdt.length as usize - header_size;

    let dsdt_region =
        unsafe { handler.map_physical_region::<u8>(dsdt.phys_address + header_size, aml_len) };

    // raw AML starts after the 36-byte SDT header
    let aml_bytes =
        unsafe { core::slice::from_raw_parts(dsdt_region.virtual_start.as_ptr(), aml_len) };

    let fixed_registers = acpi::registers::FixedRegisters::new(&fadt_mapping.get(), handler);
    let facs_region = unsafe {
        handler.map_physical_region(
            fadt_mapping.facs_address().unwrap(),
            core::mem::size_of::<acpi::sdt::facs::Facs>(),
        )
    };

    #[expect(clippy::arc_with_non_send_sync, reason = "The library is stupid")]
    let interp = acpi::aml::Interpreter::new(
        MyHandler,
        revision,
        alloc::sync::Arc::new(fixed_registers.unwrap()),
        Some(facs_region),
    );

    interp.load_table(aml_bytes).expect("failed to load DSDT");

    INTERPRETER
        .try_init_once(|| Mutex::new(interp))
        .expect("Init called once");
}

pub fn read_thermal_zones() {
    let _interp = INTERPRETER.get().expect("ACPI init called").lock();

    todo!();
}

#[derive(Debug, Clone, Copy)]
struct MyHandler;

impl acpi::Handler for MyHandler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> acpi::PhysicalMapping<Self, T> {
        log::trace!("acpi map_phys_reg: {:#0x}", physical_address);
        let phys = PhysAddr::new(physical_address as u64);
        let virt = crate::mem::phys_to_virt(phys);
        log::trace!("virt address: {:#0x}", physical_address);
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
        let mut table = ACPI_MUTEXES.lock();
        let idx = table
            .iter()
            .position(|m| !m.in_use)
            .expect("exhausted ACPI mutex lots");
        table[idx].in_use = true;
        table[idx].count = 0;
        table[idx].locked.store(false, Ordering::Relaxed);
        // Definitely not having 4 billion mutexes so this is fine
        acpi::Handle(u32::try_from(idx).unwrap())
    }

    // Acquire the mutex referred to by the given handle. `timeout` is a millisecond timeout value
    // with the following meaning:
    //    - `0` - try to acquire the mutex once, in a non-blocking manner. If the mutex cannot be
    //      acquired immediately, return `Err(AmlError::MutexAcquireTimeout)`
    //    - `1-0xfffe` - try to acquire the mutex for at least `timeout` milliseconds.
    //    - `0xffff` - try to acquire the mutex indefinitely. Should not return `MutexAcquireTimeout`.
    fn acquire(&self, mutex: acpi::Handle, timeout: u16) -> Result<(), acpi::aml::AmlError> {
        let idx = mutex.0 as usize;

        let mutex = &mut ACPI_MUTEXES.lock()[idx];

        if mutex.count > 0 {
            // Mutex is reentrant so just increment the value

            mutex.count += 1;
            return Ok(());
        }

        match timeout {
            0xffff => {
                log::error!("Indefinite acquire loop0 in acpi");
                loop {
                    core::hint::spin_loop();
                }
            }
            0 => {
                // try aquire once
                mutex
                    .locked
                    .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                    .map_err(|_| acpi::aml::AmlError::MutexAcquireTimeout)?;

                Ok(())
            }
            _ => {
                let start = time::Instant::now();

                while mutex
                    .locked
                    .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                    .is_err()
                    && start.elapsed() < Duration::from_millis(u64::from(timeout))
                {
                    core::hint::spin_loop();
                }

                mutex.count = 1;
                Ok(())
            }
        }
    }

    fn release(&self, mutex: acpi::Handle) {
        let idx = mutex.0 as usize;
        let mut table = ACPI_MUTEXES.lock();
        let mutex = &mut table[idx];

        if mutex.count > 0 {
            mutex.count -= 1;
            if mutex.count == 0 {
                mutex.locked.store(false, Ordering::Release);
            }
        }
    }
}

struct AcpiMutex {
    in_use: bool,
    locked: AtomicBool,
    /// ACPI mutexes are reentrant. That is, a thread may acquire the same mutex more than once
    /// Since everything happens on one thread, it is fine like this.
    count: u32,
}

const fn unused_mutex() -> AcpiMutex {
    AcpiMutex {
        in_use: false,
        locked: AtomicBool::new(false),
        count: 0,
    }
}

const MAX_ACPI_MUTEXES: usize = 32;

static ACPI_MUTEXES: Mutex<[AcpiMutex; MAX_ACPI_MUTEXES]> =
    Mutex::new([const { unused_mutex() }; MAX_ACPI_MUTEXES]);

fn read_addr<T>(addr: usize) -> T
where
    T: Copy,
{
    log::trace!("acpi read_addr: {:#0x}", addr);
    let virt = crate::mem::phys_to_virt(PhysAddr::new(addr as u64));
    unsafe { *virt.as_ptr() }
}
