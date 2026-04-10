#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "print_vga.h"

/* This tutorial will only work for the 32-bit ix86 targets. */
#if !defined(__i386__) && !defined(__i486__)
#error "This tutorial needs to be compiled with a ix86-elf compiler"
#endif

// https://www.gnu.org/software/grub/manual/multiboot/multiboot.html#Boot-information-format
typedef struct {
    uint32_t flags;
    uint32_t mem_lower;       // flags[0] 
    uint32_t mem_upper;       // flags[0] 

    uint32_t boot_device;     // flags[1]
    uint32_t cmdline;         // flags[2]

    uint32_t mods_count;      // flags[3]
    uint32_t mods_addr;       // flags[3]
    // requires flag[4] or flag[5]
    uint32_t syms1;
    uint32_t syms2;
    uint32_t syms3;

    uint32_t mmap_length;     // flags[6]
    uint32_t mmap_addr;       // flags[6]

    uint32_t drives_length;   // flags[7]
    uint32_t drives_addr;     // flags[7]

    uint32_t config_table;    // flags[8]

    uint32_t boot_ldr_name;  // flags[9]

    uint32_t apm_table;       // flags[10]

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

} __attribute__((packed)) BootInformationFormat ;

void kernel_main(BootInformationFormat* info) {
    /* Initialize terminal interface */
    terminal_initialize();

    uint32_t flags = info->flags;
    printbin(flags);
}
