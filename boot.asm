

CODE_START equ 0x7c00
[ORG CODE_START]
jmp start

    %include "print.asm"

start:
	xor ax, ax
	mov ds, ax
    mov es, ax
	mov ss, ax
    mov sp, 512+CODE_START

	cli

    push ds
    lgdt [gdtinfo]

    ; switch to pmode
    mov eax, cr0 
    or al, 1
    mov cr0, eax

    mov bx, 0x08
    mov ds, bx ; ds = 0b1000

    and al, 0xFE ; back to realmode??
    mov cr0, eax
    pop ds
    sti

    ; mov al, 0x02 ; smiley
    ; call cprint
    ; mov eax, 0x0b8000;
    ; mov word [ds:eax], bx

    ; jmp $

    mov ax, TEXT_VIDEO_MEMORY
    mov es, ax

    mov bx, 0x09 ; hardware interrupt 0x09. Keyboard
    shl bx, 2
    xor ax, ax
    mov gs, ax
    ; copy the address of the keyhandler routine to 0x09
    mov [gs:bx], word keyhandler
    mov [gs:bx+2], ds ; segment
    sti ; enable interrupts
    ;
    ; jmp $

gdtinfo:
   dw gdt_end - gdt - 1   ;last byte in table
   dd gdt         ;start of table


keyhandler:
    in al, 0x60 ; read from port 0x60 (96)
    mov bl, al

    mov byte [port60], al
    in al, 0x61 ; keyboard control
    mov ah, al
    or al, 0x80 ; disable bit 7
    out 0x61, al ; send it back
    xchg ah, al ; get original
    out 0x61, al ; send that back

    mov al, 0x20
    out 0x20, al

    and bl, 0x80 ; key released
    jnz .done ; don't repeat

    mov ax, [port60]
    mov word [reg32], ax
    call printreg32
.done:
    iret

    ; mov bl, al
port60   dw 0

msg db `Hello worldd whats up dawn here\nyay a new linewoahh!\0`

gdt dd 0,0
flatdesc db 0xff, 0xff, 0,0,0, 0b10010010, 0b11001111, 0
gdt_end:

times 510-($-$$) db 0
db 0x55
db 0xAA


