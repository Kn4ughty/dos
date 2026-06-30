global _start

bits 64
_start:
    mov edi, 0xb8000
    mov word [edi], 0x5502
    add edi, 2
    cmp edi, 0xb80500
    jl _start
    ret
