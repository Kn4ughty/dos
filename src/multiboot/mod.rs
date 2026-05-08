// https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html#Boot-information-format

// TODO. Write integration tests for this module

use core::mem::size_of;

#[derive(Debug)]
#[repr(C, align(8))]
pub struct BootInformationFormat {
    total_size: u32,
    _reserved: u32,
    first_tag: TagHeader,
}

impl BootInformationFormat {
    /// SAFETY
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
            total_size: self.total_size as usize,
            current: &self.first_tag as *const _,
        }
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
    total_size: usize,
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
            self.total_size < next_addr,
            "Cannot exceed total size of multiboot structure"
        );

        let result = tag;
        self.current = next_addr as *const TagHeader;

        Some(result)
    }
}

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
#[repr(C)]
pub struct MemoryMap {
    typ: u32, // Must equal 6
    size: u32,
    entry_size: u32,
    entry_version: u32,
    // This does not change the size of the struct (because its size is zero) but it does allow for
    // a marker of where the entries start.
    entries: [RawMemoryEntry; 0],
}
impl private::Sealed for MemoryMap {}

impl TagType for MemoryMap {
    const ID: u32 = 6;

    /// Returns self, if it passes some checks.
    fn validate(&self) -> Result<&Self, TagError> {
        if (self.entry_size % 8 == 0)
            && ((self.size as usize - size_of::<MemoryMap>()) % size_of::<RawMemoryEntry>() == 0)
            && (size_of::<RawMemoryEntry>() == self.entry_size as usize)
        {
            Ok(self)
        } else {
            Err(TagError::ValidationError)
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct RawMemoryEntry {
    base_addr: u64,
    length: u64,
    typ: u32,
    _reserved: u32,
}

pub struct MemoryEntry {
    /// The startin address of the memory region
    base_addr: u64,
    /// Size of the region in bytes
    length: u64,
    typ: MemoryEntryType,
}

macro_rules! TryFrom {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
        $($variant:ident = $val:expr,)*
    }) => {
        $(#[$meta])*
        $vis enum $name {
            $($variant = $val,)*
        }


        impl TryFrom<u32> for $name {
            type Error = u32;

            fn try_from(value: u32) -> Result<Self, Self::Error> {
                match value {
                    $(x if x == $name::$variant as u32 => Ok($name::$variant),)*
                    _ => Err(value)
                }
            }
        }
    }
}

TryFrom! {
    #[derive(Debug, PartialEq, Eq)]
    #[repr(u32)]
    enum MemoryEntryType {
        Available = 1,
        Reserved = 2,
        ACPIInfo = 3,
        PreserveForHibernation = 4,
        DefectiveRam = 5,
    }
}

impl TryFrom<&RawMemoryEntry> for MemoryEntry {
    type Error = ();
    fn try_from(value: &RawMemoryEntry) -> Result<Self, Self::Error> {
        let mem_type = MemoryEntryType::try_from(value.typ)
            .map_err(|_| crate::println!("[WARN] unknown MemoryEntryType: {}", value.typ))?;

        Ok(MemoryEntry {
            base_addr: value.base_addr,
            length: value.length,
            typ: mem_type,
        })
    }
}

impl MemoryMap {
    pub fn get_all_entries(&self) -> impl Iterator<Item = MemoryEntry> + '_ {
        let count = (self.size as usize - size_of::<Self>()) / self.entry_size as usize;

        let raw_slice = unsafe {
            // SAFETY: The entries start right after the end of the struct, as given in the
            // multiboot spec.
            core::slice::from_raw_parts(self.entries.as_ptr() as *const RawMemoryEntry, count)
        };

        let slice = raw_slice
            .iter()
            .filter_map(|rme| MemoryEntry::try_from(rme).ok())
            .filter(|t| t.typ == MemoryEntryType::Available);

        slice
    }
}

// Implement manually so it can be printed in hex
impl core::fmt::Debug for MemoryEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MemoryEntry")
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
