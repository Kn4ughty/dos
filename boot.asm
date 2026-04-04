jmp main

; to include: '%include "othercode.asm"'

BIOS_VIDEO_INT	    equ 0x10
; al register should contain character code
; bh should contain page number (text display screen)
DISPLAY_CHARACTER   equ 0x0E

%macro BiosPrint 1
            mov si, word %1
bios_print:
	lodsb ; load string byte
		  ; read character pointed at by SI and put into AL register
		  ; increment SI by 1
	or al, al ; Update the zero flag
	jz hang
	mov ah, DISPLAY_CHARACTER ;function  display character
	mov bh, 0
	int BIOS_VIDEO_INT
	jmp bios_print
done: ; why is this here?
%endmacro

[ORG 0x7c00]

	mov ax, 0
	mov ds, ax
	cld ; clear direction flag

main:
    BiosPrint msg

hang:
	jmp hang

					                      ;\r  \n, \0
msg db "Hello worldd whats up dawn here", 13, 10, 0


; done:
	times 510-($-$$) db 0
	db 0x55
	db 0xAA


