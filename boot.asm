
; %include "other"

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
    ; cannot be all literals
	xor ax, ax
	mov ds, ax
    mov es, ax
	mov ss, ax
    mov sp, 512+CODE_START

	cld

    mov ax, TEXT_VIDEO_MEMORY
    mov es, ax
	
	mov si, msg ; si points to msg address
	call println

    jmp $



msg db `Hello worldd whats up dawn here\nyay a new linewoahh!\0`

times 510-($-$$) db 0
db 0x55
db 0xAA


