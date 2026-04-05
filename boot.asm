

CODE_START equ 0x7c00
[ORG CODE_START]
jmp start

    %include "print.asm"

start:
    mov [boot_drive], dl ; save drive id

	xor ax, ax
	mov ds, ax
    mov es, ax
	mov ss, ax
    mov sp, 512+CODE_START

	cli


    ; https://www.ctyme.com/intr/rb-0706.htm
    mov ah, 0x41
    mov bx, 0x55AA
    mov dl, [boot_drive]
    int 0x13

    cmp bx, 0xAA55
    mov si, BX_SWAP_FAIL
    jne ERROR

    ; cx contains supported commands
    and cx, 0
    mov si, RW_FAIL
    jnz ERROR

    ; https://www.ctyme.com/intr/rb-0708.htm
    mov si, DAPACK
    mov ah, 0x42
    mov dl, [boot_drive]
    int 0x13
    mov si, READ_ERROR
    ; jump carry
    jc short ERROR


    mov al, [blkcnt]
    mov [reg16], al
    call printreg16

    mov si, next_sector_txt
    call sprint

    jmp $


; prints string at adress in si
ERROR:
    call println
    mov si, EC_PREAMBLE
    call sprint
    ; print error code: https://www.ctyme.com/intr/rb-0606.htm#Table234
    mov [reg16], ah
    call printreg16
    jmp $


gdtinfo:
   dw gdt_end - gdt - 1   ;last byte in table
   dd gdt         ;start of table

boot_drive db 0
DAPACK:
    db 0x10
    db 0
blkcnt: dw 5 ; read n blocks
db_add: 
    dw NEXT_SECTOR; memory destination
    dw 0 ; memory page
d_lba:
    dd 1 ; put the lba to read in this spot
    dd 0 ; more storage bytes only for big lba

BX_SWAP_FAIL db `bx not swapped!`, 0
RW_FAIL db `read_write not supported in cx`, 0
READ_ERROR db `Error reading drive`, 0
EC_PREAMBLE db `Error code: 0x`, 0

gdt dd 0,0
flatdesc db 0xff, 0xff, 0,0,0, 0b10010010, 0b11001111, 0
gdt_end:

times 510-($-$$) db 0
db 0x55
db 0xAA
NEXT_SECTOR:
next_sector_txt db "Successfully loaded next sector", 0

next_code:


times (512*5) db 0
