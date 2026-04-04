

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


    mov ah, 0x41
    mov bx, 0x55AA

    mov dl, 0x80
    int 0x13

    cmp bx, 0xAA55
    mov si, BX_SWAP_FAIL
    jne ERROR

    ; cx contains supported commands
    and cx, 0
    mov si, RW_FAIL
    jnz ERROR

    mov si, DAPACK
    mov ah, 0x42
    mov dl, 0x80
    int 0x13
    mov si, MYSTERY
    ; jmp ERROR
    ; jc short ERROR


    mov al, [blkcnt]
    mov [reg16], al
    call printreg16

    mov si, next_sector_txt
    call sprint

    jmp $


; prints string at adress in si
ERROR:
    call sprint
    jmp $


gdtinfo:
   dw gdt_end - gdt - 1   ;last byte in table
   dd gdt         ;start of table

    ; mov bl, al
port60   dw 0

DAPACK:
    db 0x10
    db 0
blkcnt: dw 16 ; read 16 blocks
db_add: 
    dw NEXT_SECTOR; memory destination
    dw 0 ; memory page
d_lba:
    dd 1 ; put the lba to read in this spot
    dd 0 ; more storage bytes only for big lba

BX_SWAP_FAIL db `bx not swapped!\n`, 0
RW_FAIL db `read_write not supported in cx \n`, 0
MYSTERY db `mystery\n`, 0

gdt dd 0,0
flatdesc db 0xff, 0xff, 0,0,0, 0b10010010, 0b11001111, 0
gdt_end:

times 510-($-$$) db 0
db 0x55
db 0xAA
NEXT_SECTOR:
next_sector_txt db "hello look at this!", 0
