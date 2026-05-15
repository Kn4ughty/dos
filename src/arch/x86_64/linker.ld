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
        *(.text .text.*)
    }

    .rodata : 
    {
        /* Merge all the sections together, instead of one section like per function */
        *(.rodata .rodata.*)
    }

    /* .data :  */
    /* { */
    /*     *(.data .data.*) */
    /* } */

    .data.rel.ro : {
        *(.data.rel.ro.local*) *(.data.rel.ro .data.rel.ro.*)
    }

    .bss :
    {
        *(.bss .bss.*)
    }
}
