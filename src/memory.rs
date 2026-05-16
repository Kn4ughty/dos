use crate::multiboot::{MemoryMap, MemoryRegionType};
use x86_64::{
    PhysAddr, VirtAddr,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PhysFrame, Size4KiB,
    },
};

use core::{ops::Range, panic};

/// # Safety
/// This function must be called only once to avoid aliasing `&mut`
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    unsafe {
        let level_4_pagetable = active_level_4_table(physical_memory_offset);

        OffsetPageTable::new(level_4_pagetable, physical_memory_offset)
    }
}

/// Returns a mutable reference to the active level 4 table.
///
/// # Safety
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_pagetable, _) = Cr3::read();

    let phys = level_4_pagetable.start_address();

    let virt = physical_memory_offset + phys.as_u64();

    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    unsafe { &mut *page_table_ptr }
}

/// Translates the given virtual address to its mapped physical address, or None if it is not
/// mapped.
///
/// # Safety
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
pub unsafe fn translate_addr(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    translate_addr_inner(addr, physical_memory_offset)
}

/// Limit the scope of unsafe. Must only be reachable through unsafe fn from outside this module
fn translate_addr_inner(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    use x86_64::registers::control::Cr3;
    use x86_64::structures::paging::page_table::FrameError;

    let (level_4_pagetable_frame, _) = Cr3::read();

    let table_indexes = [
        addr.p4_index(),
        addr.p3_index(),
        addr.p2_index(),
        addr.p1_index(),
    ];

    let mut frame = level_4_pagetable_frame;

    for &index in &table_indexes {
        let virt = physical_memory_offset + frame.start_address().as_u64();
        let table_ptr: *const PageTable = virt.as_ptr();
        let table = unsafe { &*table_ptr };

        let entry = &table[index];
        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => return None,
            Err(FrameError::HugeFrame) => panic!("Huge pages not supported. How'd that get there"),
        };
    }

    Some(frame.start_address() + u64::from(addr.page_offset()))
}

type FrameIterator = impl Iterator<Item = PhysFrame>;

// pub struct BootInfoFrameAllocator<I>
// where
//     I: Iterator<Item = PhysFrame>,
// {
//     memory_map: &'static MemoryMap,
//     next: usize,
//     iterator: I,
// }

pub struct BootInfoFrameAllocator {
    iterator: FrameIterator,
}

impl BootInfoFrameAllocator {
    /// Create a FrameAllocator from the passed memory mjap
    ///
    /// # Safety
    /// This function is unsafe because the caller must guarantee that the passed memory map is
    /// valid. The main requirement is that all frames that are mared `USABLE` in it are really
    /// unused.
    pub unsafe fn init(memory_map: &'static MemoryMap, reserverd_range: Range<u64>) -> Self {
        BootInfoFrameAllocator {
            iterator: Self::useable_frames(memory_map, reserverd_range),
        }
    }

    #[define_opaque(FrameIterator)]
    fn useable_frames(
        memory_map: &'static MemoryMap,
        reserverd_range: Range<u64>,
    ) -> FrameIterator {
        let regions = memory_map.get_all_entries();
        let usable_regions = regions.filter(|r| r.typ == MemoryRegionType::Available);

        let addr_ranges = usable_regions.map(|r| r.base_addr..(r.base_addr + r.length));
        let frame_addresses = addr_ranges
            .flat_map(|r| {
                assert!(r.start & 4096 == 0);
                r.step_by(4096)
            })
            .filter(move |r| !reserverd_range.contains(r)); // 4kiB pages
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        self.iterator.next()
    }
}

pub fn create_example_map(
    page: Page,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    use x86_64::structures::paging::PageTableFlags as Flags;

    let frames = PhysFrame::containing_address(PhysAddr::new(0xb8000));
    let flags = Flags::PRESENT | Flags::WRITABLE; // u64

    let map_to_result = unsafe {
        // FIXME: This is not safe since it will alias &mut. Only for testing
        mapper.map_to(page, frames, flags, frame_allocator)
    };
    map_to_result.expect("map_to failed").flush();
}

/// Align the given address `addr` upwards to alignment `align`.
///
/// Requires that `align` is a power of two.
pub fn align_up(addr: usize, align: usize) -> usize {
    debug_assert!((align & (align - 1)) == 0); // check align is power of two.
    // e.x f(0b1000) = (0b1000 & (0b0111)) == 0 = true
    // i.e Checks if only one bit is set high

    // This code is equivalent to
    /*
        let remainder = addr % align;
        if remainder == 0 {
            addr // addr already aligned
        } else {
            addr - remainder + align
        }
    */
    (addr + align - 1) & !(align - 1)
}
