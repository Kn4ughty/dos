
[BITS 16]

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
    jne SEC1ERROR
    ; cx contains supported commands
    test cx, 1
    mov si, RW_FAIL
    jz SEC1ERROR
    ; https://www.ctyme.com/intr/rb-0708.htm
    mov si, DAPACK
    mov ah, 0x42
    mov dl, [boot_drive]
    int 0x13
    mov si, READ_ERROR
    ; jump carry
    jc short SEC1ERROR

    mov al, [blkcnt]
    mov [reg16], al
    call printreg16

    mov si, next_sector_txt
    call println

    jmp next_code


; prints string at adress in si
SEC1ERROR:
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
TXT_MEM_INFO_FAIL db "Failed to read memory", 0
TXT_MEM_FOUND_PREAMBLE  db "Found 0x", 0
TXT_MEM_FOUND_POSTAMBLE db "KB of memory available", 0
TXT_MEM_TYPE db "t ", 0
TXT_MEM_BASE db       "Base address: 0x", 0
TXT_MEM_SIZE db "  Size: 0x", 0

align 4
mmap_ent:
times 24*1 db 0

get_mmap:
    ; Detect available ram
    mov ax, 0
    mov es, ax
    mov ds, ax
    mov bp, ax
    xor ebx, ebx

    mov edx, 0x534d4150 ; SMAP in ascii
    mov eax, 0xe820
    mov di, mmap_ent
    ; mov [es:di+20], dword 1
    mov ecx, 24

    int 0x15

    mov si, TXT_MEM_INFO_FAIL
    jc short .ERROR
    cmp eax, edx
    jne short .ERROR
    test ebx, ebx
    je short .ERROR

    ; print out found mmap
    mov di, mmap_ent
    call print_mem_entry

    jmp $

.ERROR:
    call println
    jmp $

; di should contain pointer to an indiviudual entry
print_mem_entry:

    mov bp, di

    mov bx, [es:bp + 16]
    mov word [reg16], bx
    call printreg16

    mov si, TXT_MEM_BASE
    call sprint

    mov eax, dword [es:bp+4]
    mov dword [reg32], eax
    call printreg32

    mov eax, dword [es:bp]
    mov dword [reg32], eax
    call printreg32

    mov si, TXT_MEM_SIZE
    call sprint

    mov eax, [es:bp+12]
    mov dword [reg32], eax
    call printreg32

    mov eax, [es:bp+8]
    mov dword [reg32], eax
    call printreg32

    mov al, `\n`
    call cprint

    ret

next_code:

    jmp get_mmap



    ; move word [reg16], [mem_type]
    ; mov eax, [mem_type]
    ; mov dword [reg32], eax
    ; call printreg32
    ; call printreg16


    jmp $



times (512*5) db 0
