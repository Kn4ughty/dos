use alloc::vec;
use alloc::vec::Vec;
use x86_64::VirtAddr;

use super::virt_to_phys;

/// Buffer of contiguous physical memory
/// useful for mmio
pub struct PhysBuf {
    pub buf: Vec<u8>,
    _private: (),
}

impl PhysBuf {
    #[must_use]
    pub fn new(len: usize) -> Self {
        // Creat ea full size buffer of correct length
        // But at this point, it is not known if the vec is contiguous
        Self::from(vec![0; len])
    }

    fn from(vec: Vec<u8>) -> Self {
        let buffer_end = vec.len() - 1;
        // Unwrap is okay since the vec is allocated by the allocator
        let phys_end = virt_to_phys(VirtAddr::from_ptr(&raw const vec[buffer_end])).unwrap();
        let phys_begin = virt_to_phys(VirtAddr::from_ptr(&raw const vec[0])).unwrap();

        if (phys_end - phys_begin) == buffer_end as u64 {
            // Yay the memory is contiguous
            Self {
                buf: vec,
                _private: (),
            }
        } else {
            // Old vec is dropped, allocate new one
            Self::from(vec.clone())
        }
    }

    #[must_use]
    #[expect(clippy::missing_panics_doc, reason = "cannot panic")]
    pub fn addr(&self) -> u64 {
        virt_to_phys(VirtAddr::from_ptr(&raw const self.buf[0]))
            .unwrap() // this cannot panic because the memory address is real
            .as_u64()
    }
}
