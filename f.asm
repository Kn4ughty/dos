global _start

bits 64
_start:
    mov edi, 0xb8000
.loop:
    mov word [edi], 0x5002
    add edi, 2
    cmp edi, 0xb8050
    jl .loop
    ret
