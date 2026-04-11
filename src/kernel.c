#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "kstdio.h"
#include "kstring.h"
#include "print_vga.h"

/* This tutorial will only work for the 32-bit ix86 targets. */
#if !defined(__i386__) && !defined(__i486__)
#error "This needs to be compiled with a ix86-elf compiler"
#endif

struct multiboot_aout_symbol_table {
        uint32_t tabsize;
        uint32_t strsize;
        uint32_t addr;
        uint32_t reserved;
};
typedef struct multiboot_aout_symbol_table multiboot_aout_symbol_table_t;
struct multiboot_elf_section_header_table {
        uint32_t num;
        uint32_t size;
        uint32_t addr;
        uint32_t shndx;
};
typedef struct multiboot_elf_section_header_table
    multiboot_elf_section_header_table_t;

// https://www.gnu.org/software/grub/manual/multiboot/multiboot.html#Boot-information-format
typedef struct {
        uint32_t flags;
        uint32_t mem_lower; // flags[0]
        uint32_t mem_upper; // flags[0]

        uint32_t boot_device; // flags[1]

        uint32_t cmdline; // flags[2]

        uint32_t mods_count; // flags[3]
        uint32_t mods_addr;  // flags[3]

        // requires flag[4] or flag[5]
        // uint32_t syms1;
        // uint32_t syms2;
        // uint32_t syms3;
        union {
                multiboot_aout_symbol_table_t aout_sym;
                multiboot_elf_section_header_table_t elf_sec;
        } u;

        uint32_t mmap_length; // flags[6]
        uint32_t mmap_addr;   // flags[6]

        uint32_t drives_length; // flags[7]
        uint32_t drives_addr;   // flags[7]

        uint32_t config_table; // flags[8]

        uint32_t boot_ldr_name; // flags[9]

        uint32_t apm_table; // flags[10]

        // all present if flags[11] is set
        uint32_t vbe_control_info;
        uint32_t vbe_mode_info;
        uint32_t vbe_mode;
        uint32_t vbe_interface_seg;
        uint32_t vbe_interface_off;
        uint32_t vbe_interface_len;

        // all present if flags[12] is set
        uint32_t framebuffer_addr;
        uint32_t framebuffer_pitch;
        uint32_t framebuffer_width;
        uint32_t framebuffer_height;
        uint32_t framebuffer_bpp;
        // 1 byte,
        // 5 bytes

} __attribute__((packed)) BootInformationFormat;

typedef struct {
        uint32_t size;
        uint64_t base_addr;
        uint64_t length;
        uint32_t type;
} __attribute__((packed)) MmapEntry;

void kernel_main(BootInformationFormat *info)
{
        /* Initialize terminal interface */
        terminal_initialize();
        init_serial();

        // printhex((uint32_t)info);
        // puts(SV("\n"));
        // printhex(info->flags);

        uint32_t flags = info->flags;
        if (!((flags >> 6) & 1)) {
                // panic!
                return;
        }

        // uint8_t *mmap_ptr = (uint8_t *)info->mmap_addr;

        puts(SV("Memory map flag is set!\n"));
        puts(SV("\n"));

        uint32_t total_processed = 0;
        uint8_t *mmap_ptr = (uint8_t *)(uintptr_t)info->mmap_addr;

        // while (total_processed < info->mmap_length) {
        for (MmapEntry *entry = (MmapEntry *)(uintptr_t)info->mmap_addr;
             (uintptr_t)entry < (info->mmap_addr + info->mmap_length);
             entry = (MmapEntry *)((uint8_t *)entry + entry->size + 4)) {

                // while (total_processed < 3) {
                MmapEntry *entry = (MmapEntry *)(mmap_ptr);
                // puts(SV("t: "));
                // printhex32((uint32_t)entry->type);
                printf(SV("t: %d"), entry->type);

                printf(SV(" len: 0x%x"), entry->length);

                printf(SV(" base_addr: 0x%x"), entry->base_addr);

                puts(SV("\n"));
                // printhex(mmap->length);
                // break;
                uint32_t size = entry->size + sizeof(entry->size);
                mmap_ptr += size;
                total_processed += size;
        }
        //
        // StringView s = sv("test");
}
