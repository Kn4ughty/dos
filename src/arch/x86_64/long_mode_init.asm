global long_mode_start

%include "constants.asm"

section .text
bits 64
long_mode_start:
    mov ax, 0
    mov ss, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    mov rsi, PAGE_TABLE_OFFSET

    extern rust_main
    call rust_main

    mov rax, 0x2f592f412f4b2f4f
    mov qword [0xb8000], rax
    hlt

section .note.GNU-stack noalloc noexec nowrite progbits
