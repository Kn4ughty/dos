
BIOS_VIDEO_INT	    equ 0x10
; al register should contain character code
; bh should contain page number (text display screen)
DISPLAY_CHARACTER   equ 0x0E

TEXT_VIDEO_MEMORY equ 0xb800


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
	
    jmp $


keyhandler:
    in al, 0x60 ; read from port 0x60 (96)
    movzx ax, al
    mov word [reg16], ax
    call printreg16
    ret

    ; mov bl, al

msg db `Hello worldd whats up dawn here\nyay a new linewoahh!\0`

times 510-($-$$) db 0
db 0x55
db 0xAA


