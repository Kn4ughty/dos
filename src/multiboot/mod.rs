// https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html#Boot-information-format

// TODO. Write integration tests for this module

mod memory_map;
pub use memory_map::{MemoryEntry, MemoryMap, MemoryRegionType};
mod elf;
pub use elf::{ELFSymbols, ElfSection};

mod private {
    pub trait Sealed {}
}

pub trait TagType: private::Sealed {
    const ID: u32;

    /// For Tags to implement their own validation methods.
    /// Override with correct implementation when needed.
    fn validate(&self) -> Result<&Self, TagError> {
        Ok(self)
    }
}

#[derive(Debug)]
#[repr(C, align(8))]
pub struct BootInformationFormat {
    total_size: u32,
    _reserved: u32,
    first_tag: TagHeader,
}

impl BootInformationFormat {
    /// # Safety
    /// Caller must pass in valid address to the multiboot2 header.
    pub unsafe fn load<'a>(addr: usize) -> &'a BootInformationFormat {
        assert_eq!(addr % 8, 0, "Multiboot Header must be 8-byte aligned");

        let multiboot = unsafe { &*(addr as *const BootInformationFormat) };

        assert_eq!(
            multiboot.total_size % 8,
            0,
            "Total size must be 8b aligned. (every tag is 8 byte alligned, so it follows the size must be too)"
        );

        multiboot
    }

    pub fn get<T: TagType>(&self) -> Result<&T, TagError> {
        self.tags()
            .find(|t| t.typ == T::ID)
            // SAFETY. Should be the correct type, as the ID matches for T
            .map(|tag| unsafe { &*(tag as *const TagHeader as *const T) })
            .ok_or(TagError::NotFound)
            .and_then(|tag| tag.validate())
    }

    fn tags(&self) -> TagIter {
        TagIter {
            end_address: self as *const BootInformationFormat as usize + self.total_size as usize,
            current: &self.first_tag as *const TagHeader,
        }
    }

    pub fn start_addr(&self) -> u64 {
        self as *const Self as u64
    }

    pub fn end_addr(&self) -> u64 {
        self.start_addr() + self.total_size as u64
    }
}

#[derive(Debug)]
pub enum TagError {
    NotFound,
    ValidationError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, align(8))]
struct TagHeader {
    typ: u32,
    size: u32,
    // Additional fields...
}

#[derive(Debug)]
struct TagIter {
    end_address: usize,
    current: *const TagHeader,
}

impl Iterator for TagIter {
    type Item = &'static TagHeader;

    fn next(&mut self) -> Option<&'static TagHeader> {
        // SAFETY. Since previous iteration should have set as valid tag its okay
        let tag = unsafe { &*self.current };

        assert!(
            tag.size >= 8,
            "Tags cannot be less than 8 bytes in size, since that would be smaller than the header"
        );

        if tag.typ == 0 && tag.size == 8 {
            return None;
        }

        let mut next_addr = self.current as usize;

        // Align with padding to next 8 bytes
        next_addr += ((tag.size + 7) & !0x7) as usize;
        // This basically just checks that the padding math is correct, so its just for debug mode.
        debug_assert_eq!(next_addr % 8, 0, "Tags must be 8 byte aligned.");

        // This assert _should_ only trigger if the end tag is missing, or a tag is marked with an
        // incorrect (and large) size.
        assert!(
            next_addr <= self.end_address,
            "Cannot exceed total size of multiboot structure"
        );

        let result = tag;
        self.current = next_addr as *const TagHeader;

        Some(result)
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct BootLoaderName {
    header: TagHeader,
    string_start: [core::ffi::c_char; 0],
}

impl private::Sealed for BootLoaderName {}

impl TagType for BootLoaderName {
    const ID: u32 = 2;
}

impl BootLoaderName {
    pub fn name(&self) -> Result<&str, core::str::Utf8Error> {
        let ptr = self.string_start.as_ptr();

        // SAFETY: The multiboot spec requires that the string have a null terminator.
        unsafe { core::ffi::CStr::from_ptr(ptr).to_str() }
    }
}
