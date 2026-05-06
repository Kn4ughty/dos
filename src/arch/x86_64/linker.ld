ENTRY(_start)

SECTIONS {
    . = 1M;

    .boot :
    {
        /* ensure that the multiboot header is at the beginning */
        *(.multiboot_header)
    }

    .text :
    {
        *(.text)
    }

    .rodata : 
    {
        *(.rodata .rodata.*)
    }

    .data : 
    {
        *(.data .data.*)
    }
    .bss :
    {
        *(.bss .bss.*)
    }
}
