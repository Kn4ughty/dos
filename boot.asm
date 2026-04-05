
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
    and cx, 0
    mov si, RW_FAIL
    jnz SEC1ERROR
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

MEM_INFO:
mem_baseaddr dq 0
mem_length dq 0
mem_type dd 0
mem_eabf dd 0

mmap_ent equ 0x8000

get_mmap:
    ; ; Detect available ram
    mov ax, 0
    mov es, ax
    mov ds, ax
    mov bp, ax
    xor ebx, ebx

    mov eax, 0xe820
    xor ebx, ebx
    mov ecx, 24
    mov edx, 0x534d4150 ; SMAP in ascii
    mov di, MEM_INFO

    int 0x15

    mov si, TXT_MEM_INFO_FAIL
    jc short .ERROR
    cmp eax, edx
    jne short .ERROR
    test ebx, ebx

    jmp print_mem_entry
    jmp $

.ERROR:
    call println
    jmp $

; di should contain pointer to an indiviudual entry
print_mem_entry:
    mov di, MEM_INFO


    movzx ax, cl 
    mov word [reg16], ax
    call printreg16

    mov si, TXT_MEM_TYPE
    call sprint
    mov ax, [MEM_INFO+16]
    mov word [reg16], ax
    call printreg16

    mov si, TXT_MEM_BASE
    call sprint
    mov eax, [MEM_INFO+4]
    mov dword [reg32], eax
    call printreg32
    mov eax, [MEM_INFO]
    mov dword [reg32], eax
    call printreg32

    mov si, TXT_MEM_SIZE
    call sprint
    mov eax, [MEM_INFO+12]
    mov dword [reg32], eax
    call printreg32
    mov eax, [MEM_INFO+8]
    mov dword [reg32], eax
    call printreg32

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
