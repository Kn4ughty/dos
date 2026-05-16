use super::{TagError, TagType, private::Sealed, tryfrom};

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
impl Sealed for MemoryMap {}

impl TagType for MemoryMap {
    const ID: u32 = 6;

    /// Returns self, if it passes some checks.
    fn validate(&self) -> Result<&Self, TagError> {
        if self.entry_size.is_multiple_of(8)
            && (self.size as usize - size_of::<MemoryMap>())
                .is_multiple_of(size_of::<RawMemoryEntry>())
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

#[derive(Clone)]
pub struct MemoryEntry {
    /// The startin address of the memory region
    pub base_addr: u64,
    /// Size of the region in bytes
    pub length: u64,
    pub typ: MemoryRegionType,
}

tryfrom! {
    #[derive(Debug, PartialEq, Eq, Clone)]
    #[repr(u32)]
    pub enum MemoryRegionType {
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
        let mem_type = MemoryRegionType::try_from(value.typ)
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
            core::slice::from_raw_parts(self.entries.as_ptr(), count)
        };

        raw_slice
            .iter()
            .filter_map(|rme| MemoryEntry::try_from(rme).ok())
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
