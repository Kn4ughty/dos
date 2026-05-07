// https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html#Boot-information-format

// TODO. Write integration tests for this module

use core::mem::size_of;
use core::ptr;

#[derive(Debug)]
#[repr(C, align(8))]
pub struct BootInformationFormat {
    total_size: u32,
    _reserved: u32,
    first_tag: TagHeader,
}

impl BootInformationFormat {
    /// Safety
    /// Must be valid address to BootInformationFormat
    pub unsafe fn load<'a>(addr: usize) -> &'a BootInformationFormat {
        let multiboot = unsafe { &*(addr as *const BootInformationFormat) };
        assert_eq!(multiboot.total_size % 8, 0);
        multiboot
    }

    // TODO. return results
    pub fn get_bootloader_name(&self) -> Option<&BootLoaderName> {
        self.tags()
            .find(|t| t.typ == const { TagType::BootLoaderName as u32 })
            .map(|tag| unsafe { &*(tag as *const TagHeader as *const BootLoaderName) })
    }

    pub fn get_memory_map(&self) -> Option<&MemoryMap> {
        self.tags()
            .find(|t| t.typ == const { TagType::MemoryMap as u32 })
            .map(|tag| unsafe { &*(tag as *const TagHeader as *const MemoryMap) })
            .map(|mm| mm.validate())
            .flatten()
    }

    fn tags(&self) -> TagIter {
        let t = TagIter {
            current: &self.first_tag as *const _,
        };
        assert_eq!(unsafe { *t.current }, self.first_tag);
        t
    }
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
    current: *const TagHeader,
}

impl Iterator for TagIter {
    type Item = &'static TagHeader;

    fn next(&mut self) -> Option<&'static TagHeader> {
        // SAFETY. Since previous iteration should have set as valid tag its okay
        let tag = unsafe { &*self.current };

        assert!(tag.size >= 8);

        if tag.typ == const { TagType::End as u32 } && tag.size == 8 {
            return None;
        }

        let mut next_addr = self.current as usize;
        next_addr += ((tag.size + 7) & !0x7) as usize;
        assert_eq!(next_addr % 8, 0);

        let result = tag;
        self.current = next_addr as *const TagHeader;

        Some(result)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
#[non_exhaustive]
#[repr(u32)]
enum TagType {
    // BasicMemoryInfo = 4, // size = 16
    End = 0,
    MemoryMap = 6,
    BootLoaderName = 2,
}

#[derive(Debug)]
#[repr(C)]
pub struct MemoryMap {
    typ: u32, // Must equal 6
    size: u32,
    entry_size: u32,
    entry_version: u32,
    // This does not change the size of the struct (because its size is zero) but it does allow for
    // a marker of where the entries start.
    entries: [MemoryEntry; 0],
}

#[repr(C)]
pub struct MemoryEntry {
    base_addr: u64,
    length: u64, // Size of region in bytes
    typ: u32,
    _reserved: u32,
}

impl MemoryMap {
    /// Returns self, if it passes some checks.
    fn validate(&self) -> Option<&Self> {
        if (self.entry_size % 8 == 0)
            && ((self.size as usize - size_of::<MemoryMap>()) % size_of::<MemoryEntry>() == 0)
            && (size_of::<MemoryEntry>() == self.entry_size as usize)
        {
            Some(self)
        } else {
            None
        }
    }

    pub fn get_all_entries(&self) -> &[MemoryEntry] {
        let count = (self.size as usize - size_of::<Self>()) / self.entry_size as usize;

        unsafe {
            // SAFETY: The entries start right after the end of the struct, as given in the
            // multiboot spec.
            core::slice::from_raw_parts(self.entries.as_ptr() as *const MemoryEntry, count)
        }
    }
}

impl core::fmt::Debug for MemoryEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MemoryEntry")
            // 16 nibbles needed for u64
            .field("base_addr", &format_args!("{:#X}", self.base_addr))
            .field("length", &format_args!("{:#X}", self.length))
            .field("type", &self.typ)
            .finish()
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct BootLoaderName {
    header: TagHeader,
    string_start: [core::ffi::c_char; 0],
}

impl BootLoaderName {
    pub fn name(&self) -> Result<&str, core::str::Utf8Error> {
        let ptr = self.string_start.as_ptr();

        // SAFETY: The multiboot spec requires that the string have a null terminator.
        unsafe { core::ffi::CStr::from_ptr(ptr).to_str() }
    }
}
