// https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html#Boot-information-format

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
    pub fn get_bootloader_name(&self) -> &BootLoaderName {
        let basic_tag = self
            .tags()
            .find(|t| t.typ == const { TagType::BootLoaderName as u32 })
            .expect("Could not find bootloader name");

        // unsafe { core::mem::transmute(basic_tag) }

        return unsafe { &*(basic_tag as *const TagHeader as *const BootLoaderName) };
    }

    pub fn get_memory_map(&self) -> &MemoryMap {
        let base_tag = self
            .tags()
            .find(|t| t.typ == const { TagType::MemoryMap as u32 })
            .expect("COuld find mem map");

        let mmap = unsafe { &*(base_tag as *const TagHeader as *const MemoryMap) };
        mmap.validate();
        mmap
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

    // /*
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
    fn validate(&self) {
        assert_eq!(self.entry_size % 8, 0);
        // size is mulitple of entry size
        assert_eq!(
            (self.size as usize - size_of::<MemoryMap>()) % size_of::<MemoryEntry>(),
            0
        );

        assert_eq!(size_of::<MemoryEntry>(), self.entry_size as usize); // lol. This is not compatable with the spec.
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
    // String start address goes here!
    // Boot loaded name is here. But Structs have to have a static size, so
    // I cant just go [char]. This means I need to do a manual offset. Doing a
    // single `string_start: u8` doesnt work, since then for some reason it only
    // finds the first byte of the string.
}

impl BootLoaderName {
    pub fn name(&self) -> Result<&str, core::str::Utf8Error> {
        unsafe {
            // Offset pointer to start of string
            let ptr = (self as *const Self as *const u8).add(size_of::<Self>());

            let max_len = self.header.size as usize;

            let slice = core::slice::from_raw_parts(ptr, max_len);

            let len = slice.iter().position(|b| *b == 0).unwrap_or(max_len);
            core::str::from_utf8(&slice[..len])
        }
    }
}
