/// 🧝
/// elf
///  /\*
/// \ᵔᵕᵔ/
///  ||
///  /\
use super::{TagHeader, TagType, private::Sealed, tryfrom};

// https://wiki.osdev.org/ELF

// The spec actually lies about this tag. It says the tag is this:
/*      +-------------------+
u32     | type = 9          |
u32     | size              |
u16     | num               |
u16     | entsize           |
u16     | shndx             |
u16     | reserved          |
varies  | section headers   |
        +-------------------+

then in the c code example it says:
struct multiboot_tag_elf_sections
{
  multiboot_uint32_t type;
  multiboot_uint32_t size;
  multiboot_uint32_t num;
  multiboot_uint32_t entsize;
  multiboot_uint32_t shndx;
  char sections[0];
};
*/
// The c code appears to be the actually correct representation.
#[derive(Debug)]
#[repr(C, align(8))]
pub struct ELFSymbols {
    header: TagHeader,
    num_sections: u32,
    entry_size: u32,
    string_section_header_index: u32,
    section_headers: [(); 0],
}

impl Sealed for ELFSymbols {}
impl TagType for ELFSymbols {
    const ID: u32 = 9;
}

impl ELFSymbols {
    pub fn get_sections(&self) -> impl Iterator<Item = ElfSection> {
        let headers_ptr = self.section_headers.as_ptr() as *const ElfSectionInner64;
        let sthp = unsafe { headers_ptr.add(self.string_section_header_index as usize) };

        ElfSectionIter {
            current_section: self.section_headers.as_ptr() as *const _,
            string_table_header: sthp as *const _,
            entry_size: self.entry_size,
            remaining_sections: self.num_sections,
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ElfSection {
    inner: *const u8,
    string_section: *const u8,
    entry_size: u32,
}

impl ElfSection {
    // use dyn in future to support 32 bit elf
    fn inner(&self) -> &ElfSectionInner64 {
        unsafe { &*(self.inner as *const ElfSectionInner64) }
    }

    pub fn name(&self) -> Result<&str, core::str::Utf8Error> {
        let shdr = unsafe { &*(self.string_section as *const ElfSectionInner64) };

        let stringtable_ptr = shdr.addr as *const i8;

        let name_ptr = unsafe { stringtable_ptr.offset(self.inner().name_index as isize) };

        unsafe { core::ffi::CStr::from_ptr(name_ptr).to_str() }
    }

    pub fn start_addr(&self) -> u64 {
        self.inner().addr
    }

    pub fn end_addr(&self) -> u64 {
        self.inner().addr + self.inner().size
    }

    pub fn section_type(&self) -> SectionType {
        SectionType::try_from(self.inner().typ).unwrap_or(SectionType::Unknown)
    }
}

#[derive(Debug)]
#[repr(C)]
struct ElfSectionIter {
    current_section: *const u8,
    remaining_sections: u32,
    entry_size: u32,
    string_table_header: *const u8,
}

impl Iterator for ElfSectionIter {
    type Item = ElfSection;

    fn next(&mut self) -> Option<ElfSection> {
        while self.remaining_sections != 0 {
            let section = ElfSection {
                inner: self.current_section,
                string_section: self.string_table_header,
                entry_size: self.entry_size,
            };

            self.current_section = unsafe { self.current_section.offset(self.entry_size as isize) };
            self.remaining_sections -= 1;

            if section.section_type() != SectionType::Inactive {
                return Some(section);
            }
        }
        None
    }
}

// https://docs.oracle.com/cd/E23824_01/html/819-0690/chapter6-94076.html
// #[derive(Debug)]
// #[repr(C, packed)]
// struct ElfSectionInner64 {
//     name_index: u32,
//     typ: u32,
//     flags: u64,
//     addr: u64,
//     offset: u64,
//     size: u64,
//     link: u64,
//     info: u64,
//     addralign: u64,
//     entsize: u64,
// }

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
struct ElfSectionInner64 {
    name_index: u32,
    typ: u32,
    flags: u64,
    addr: u64,
    offset: u64,
    size: u64,
    link: u32,
    info: u32,
    addralign: u64,
    entry_size: u64,
}

// https://docs.oracle.com/cd/E23824_01/html/819-0690/chapter6-94076.html#chapter6-73445
tryfrom! {
    #[derive(Debug, PartialEq, Eq)]
    #[repr(u32)]
    pub enum SectionType {
        Inactive = 0,
        Progbits = 1,
        SymTable = 2,
        StringTable = 3,
        Rela = 4,
        Hash = 5,
        Dynamic = 6,
        Note = 7,
        NoBits = 8,
        Rel = 9,
        Unknown = u32::MAX,
        // Who cares about the rest (not me)
    }
}
