// https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html#Boot-information-format

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct BootInformationFormat {
    total_size: u32,
    _reserved: u32,
    first_tag: Tag,
}

impl BootInformationFormat {
    /// Safety
    /// Must be valid address to BootInformationFormat
    pub unsafe fn load(addr: usize) -> BootInformationFormat {
        let multiboot = unsafe { *(addr as *const BootInformationFormat) };
        // TODO. Any assertions
        multiboot
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct Tag {
    typ: u32,
    size: u32,
    // Additional fields...
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
#[non_exhaustive]
#[repr(u32)]
enum TagType {
    // BasicMemoryInfo = 4, // size = 16
    MemoryMap = 6,
    BootLoaderName = 2,
}

#[repr(C)]
struct MemoryMap {
    typ: u32, // Must equal 6
    size: u32,
    entry_size: u32, // Size of one entry. size % 8 == 0
    entry_version: u32,
    first_entry: MemoryEntry,
}

#[repr(C)]
struct MemoryEntry {
    base_addr: u64,
    length: u64, // Size of region in bytes
    typ: u32,
    _reserved: u32,
}

#[repr(C)]
struct BootLoaderName {
    typ: u32,
    size: u32,
    string: *const [char], // UTF-8 Cstr
}
