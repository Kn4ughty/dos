jmp main

; %include "other"

BIOS_VIDEO_INT	    equ 0x10
; al register should contain character code
; bh should contain page number (text display screen)
DISPLAY_CHARACTER   equ 0x0E

TEXT_VIDEO_MEMORY equ 0xb800


CODE_START equ 0x7c00
[ORG CODE_START]

    ; cannot be all literals
	xor ax, ax
	mov ds, ax
    mov es, ax
	mov ss, ax
    ; mov sp, 0x200+CODE_START
    mov sp, CODE_START

	cld ; clear direction flag

main:
    mov ax, TEXT_VIDEO_MEMORY
    mov es, ax

	mov si, msg ; si points to msg address
    call println

    ; call bios_print
    jmp hang

hang:
	jmp hang

dochar: 
    call cprint
; Put address of line in si
println:
    lodsb
    cmp al, 0
    jne dochar
    add byte [ypos], 1  ; down one row
    mov byte [xpos], 0  ; back to left of screen 
    ret

cprint:
    cmp al, 10
    jne .draw_char
    add byte [ypos], 1
    mov byte [xpos], 0
    ret
.draw_char:
    mov ah, 0x0F ; attrib = white on black

    push ax

    movzx ax, byte [ypos]
    mov dx, 160 ; total length of bytes (80cols * stride of 2)
    mul dx ; y pos * 160.

    movzx bx, byte [xpos]
    shl bx, 1 ; bx = x * 2

    mov di, 0
    add di, ax
    add di, bx ; di = (y*80*2 + xpos*2) 

    pop ax
    ;store string word
    stosw ; Copy value of ax into address of di
          ; Then increment di by 2
    add byte [xpos], 1

    ret


bios_print:
	lodsb ; load string byte
		  ; read character pointed at by SI and put into AL register
		  ; increment SI by 1
	or al, al ; Update the zero flag
	jz .exit
	mov ah, DISPLAY_CHARACTER ;function  display character
	mov bh, 0
	int BIOS_VIDEO_INT
	jmp bios_print

.exit:
    ret

xpos db 0
ypos db 0
msg db `Hello worldd whats up dawn here\nyay a new linewoahh!\0`

times 510-($-$$) db 0
db 0x55
db 0xAA


