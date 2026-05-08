global _start
extern long_mode_start

section .text
bits 32
_start:
    mov esp, stack_top
    mov edi, ebx ; Move multiboot info pointer

    call check_multiboot
    call check_cpuid
    call check_long_mode

    call setup_page_tables
    call enable_paging

    lgdt [gdt64.pointer]

    jmp gdt64.code:long_mode_start ; jump to 64bit land

    mov dword [0xb8000], 0x4f554f4E
    hlt

check_multiboot:
    cmp eax, 0x36d76289
    jne .no_multiboot
    ret
.no_multiboot:
    mov al, "0"
    jmp error

; Source: https://wiki.osdev.org/Setting_Up_Long_Mode#Detection_of_CPUID
; it may be preferrable to put this in a separate file to be included,
; along with any other EFLAGS bits you may want to use
EFLAGS_ID equ 1 << 21           ; if this bit can be flipped, the CPUID
                                ; instruction is available

; Checks if CPUID is supported by attempting to flip the ID bit (bit 21) in
; the EFLAGS register. If we can flip it, CPUID is available.
; returns eax = 1 if there is cpuid support; 0 otherwise
check_cpuid:
    pushfd
    pop eax

    ; The original value should be saved for comparison and restoration later
    mov ecx, eax

    xor eax, EFLAGS_ID

    ; storing the eflags and then retrieving it again will show whether or not
    ; the bit could successfully be flipped
    push eax                    ; save to eflags
    popfd

    pushfd                      ; restore from eflags
    pop eax

    ; Restore EFLAGS to its original value
    push ecx
    popfd

    ; if the bit in eax was successfully flipped (eax != ecx), CPUID is supported.
    xor eax, ecx
    jz .notSupported
    ret

.notSupported:
    mov al, "1"
    jmp error

CPUID_EXTENSIONS equ 0x80000000 ; returns the maximum extended requests for cpuid
CPUID_EXT_FEATURES equ 0x80000001 ; returns flags containing long mode support among other things
CPUID_EDX_EXT_FEAT_LM equ 1 << 29   ; if this is set, the CPU supports long mode
check_long_mode:
    mov eax, CPUID_EXTENSIONS
    cpuid
    cmp eax, CPUID_EXT_FEATURES
    jb .no_long_mode

    mov eax, CPUID_EXT_FEATURES
    cpuid
    test edx, CPUID_EDX_EXT_FEAT_LM
    jz .no_long_mode
    ret

.no_long_mode:
    mov al, "2"
    jmp error

setup_page_tables:
    mov eax, p3_table
    or eax, 0b11
    mov [p4_table], eax

    mov eax, p2_table
    or eax, 0b11;
    mov [p3_table], eax

    ; map each p2 entry to huge 2MiB page
    mov ecx, 0 ; counter
.map_p2_table:
    mov eax, 0x200000 ; 2MiB
    mul ecx
    or eax, 0b10000011 ; present + writable + huge
    mov [p2_table+ecx*8], eax

    inc ecx
    cmp ecx, 512
    jne .map_p2_table

    ret

enable_paging:
    mov eax, p4_table
    mov cr3, eax

    mov eax, cr4
    or eax, 1<< 5
    mov cr4, eax

    mov ecx, 0xC0000080
    rdmsr
    or eax, 1<< 8
    wrmsr

    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax

    ret

error:
    mov dword [0xb8000], 0x4f524f45
    mov dword [0xb8004], 0x4f3a4f52
    mov dword [0xb8008], 0x4f204f20
    mov byte  [0xb800a], al
    hlt


section .bss
align 4096
align 4096
p4_table:
    resb 4096
p3_table:
    resb 4096
p2_table:
    resb 4096
stack_bottom:
    resb 4096 * 4
stack_top:

section .rodata
gdt64:
    dq 0
.code: equ $ - gdt64
    dq (1<<43) | (1<<44) | (1<<47) | (1<<53)
.pointer:
    dw $ - gdt64 - 1
    dq gdt64

section .note.GNU-stack noalloc noexec nowrite progbits
