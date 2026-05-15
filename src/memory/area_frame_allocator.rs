use super::{Frame, FrameAllocator};
use crate::multiboot::{MemoryEntry, MemoryMap};

pub struct AreaFrameAllocator<'a> {
    next_free_frame: Frame,
    current_area: Option<MemoryEntry>,
    areas: &'a MemoryMap,
    kernel_start: Frame,
    kernel_end: Frame,
    multiboot_start: Frame,
    multiboot_end: Frame,
}

impl FrameAllocator for AreaFrameAllocator<'_> {
    fn allocate_frame(&mut self) -> Option<Frame> {
        let Some(ref area) = self.current_area else {
            // No free frames left
            return None;
        };

        let frame = Frame {
            number: self.next_free_frame.number,
        };

        let current_area_last_frame = {
            let address = area.base_addr + area.length - 1;
            Frame::containing_address(address as usize)
        };

        if frame > current_area_last_frame {
            // We have used up all the feames of the current memory area!
            self.choose_next_area();
        } else if frame >= self.kernel_start && frame <= self.kernel_end {
            // This frame is used by the kernel. Skip to end
            self.next_free_frame = Frame {
                number: self.kernel_end.number + 1,
            };
        } else if frame >= self.multiboot_start && frame <= self.multiboot_end {
            // This frame is used by the kernel. Skip to end
            self.next_free_frame = Frame {
                number: self.multiboot_end.number + 1,
            };
        } else {
            // Current frame is unused.
            self.next_free_frame.number += 1;
            return Some(frame);
        }
        // Frame was not valid. Try again
        self.allocate_frame()
    }
    fn deallocate_frame(&mut self, _frame: Frame) {
        unimplemented!()
    }
}

impl AreaFrameAllocator<'_> {
    fn choose_next_area(&mut self) {
        self.current_area = self
            .areas
            .get_all_entries()
            .filter(|area| {
                let address = area.base_addr + area.length - 1;
                Frame::containing_address(address as usize) >= self.next_free_frame
            })
            .min_by_key(|area| area.base_addr);

        if let Some(area) = &self.current_area.clone() {
            let start_frame = Frame::containing_address(area.base_addr as usize);
            if self.next_free_frame < start_frame {
                self.next_free_frame = start_frame;
            }
        }
    }
}

pub fn new(
    k_start: usize,
    k_end: usize,
    multiboot_start: usize,
    multiboot_end: usize,
    memory_areas: &MemoryMap,
) -> AreaFrameAllocator {
    let mut allocator = AreaFrameAllocator {
        next_free_frame: Frame::containing_address(0),
        current_area: None,
        areas: memory_areas,
        kernel_start: Frame::containing_address(k_start),
        kernel_end: Frame::containing_address(k_end),
        multiboot_start: Frame::containing_address(multiboot_start),
        multiboot_end: Frame::containing_address(multiboot_end),
    };
    allocator.choose_next_area();
    allocator
}
