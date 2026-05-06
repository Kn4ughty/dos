// https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html#Boot-information-format

use core::usize;

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

    pub fn get_bootloader_name(&self) -> &BootLoaderName {
        let basic_tag = self
            .tags()
            .find(|t| t.typ == const { TagType::BootLoaderName as u32 })
            .expect("Could not find bootloader name");

        // unsafe { core::mem::transmute(basic_tag) }

        return unsafe { &*(basic_tag as *const TagHeader as *const BootLoaderName) };
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
            let mut ptr = (self as *const Self as *const u8).add(core::mem::size_of::<Self>());

            let max_len = self.header.size as usize;

            let slice = core::slice::from_raw_parts(ptr, max_len);

            let len = slice.iter().position(|b| *b == 0).unwrap_or(max_len);
            core::str::from_utf8(&slice[..len])
        }
    }
}
