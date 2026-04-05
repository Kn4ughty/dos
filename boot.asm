
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
    ; mov sp, 512+CODE_START
    mov sp, 0x2000

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


; gdtinfo:
;    dw gdt_end - gdt - 1   ;last byte in table
;    dd gdt         ;start of table

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

; gdt dd 0,0
; flatdesc db 0xff, 0xff, 0,0,0, 0b10010010, 0b11001111, 0
; gdt_end:

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
times 24*20 db 0
mmap_end:

; code from: https://wiki.osdev.org/Detecting_Memory_(x86)#Getting_an_E820_Memory_Map
; use the INT 0x15, eax= 0xE820 BIOS function to get a memory map
; note: initially di is 0, be sure to set it to a value so that the BIOS code will not be overwritten. 
;       The consequence of overwriting the BIOS code will lead to problems like getting stuck in `int 0x15`
; inputs: es:di -> destination buffer for 24 byte entries
; outputs: bp = entry count, trashes all registers except esi
; mmap_ent equ 0x8000             ; the number of entries will be stored at 0x8000
do_e820:
    mov di, mmap_ent          ; Set di to 0x8004. Otherwise this code will get stuck in `int 0x15` after some entries are fetched 
	xor ebx, ebx		; ebx must be 0 to start
	xor bp, bp		; keep an entry count in bp
	mov edx, 0x0534D4150	; Place "SMAP" into edx
	mov eax, 0xe820
	mov [es:di + 20], dword 1	; force a valid ACPI 3.X entry
	mov ecx, 24		; ask for 24 bytes
	int 0x15
	jc short .failed	; carry set on first call means "unsupported function"
	mov edx, 0x0534D4150	; Some BIOSes apparently trash this register?
	cmp eax, edx		; on success, eax must have been reset to "SMAP"
	jne short .failed
	test ebx, ebx		; ebx = 0 implies list is only 1 entry long (worthless)
	je short .failed
	jmp short .jmpin
.e820lp:
	mov eax, 0xe820		; eax, ecx get trashed on every int 0x15 call
	mov [es:di + 20], dword 1	; force a valid ACPI 3.X entry
	mov ecx, 24		; ask for 24 bytes again
	int 0x15
	jc short .e820f		; carry set means "end of list already reached"
	mov edx, 0x0534D4150	; repair potentially trashed register
.jmpin:
	jcxz .skipent		; skip any 0 length entries
	cmp cl, 20		; got a 24 byte ACPI 3.X response?
	jbe short .notext
	test byte [es:di + 20], 1	; if so: is the "ignore this data" bit clear?
	je short .skipent
.notext:
	mov ecx, [es:di + 8]	; get lower uint32_t of memory region length
	or ecx, [es:di + 12]	; "or" it with upper uint32_t to test for zero
	jz .skipent		; if length uint64_t is 0, skip entry
	inc bp			; got a good entry: ++count, move to next storage spot
	add di, 24
.skipent:
	test ebx, ebx		; if ebx resets to 0, list is complete
	jne short .e820lp
.e820f:
	mov [es:mmap_ent], bp	; store the entry count
	clc			; there is "jc" on end of list to this point, so the carry must be cleared
	ret
.failed:
	stc			; "function unsupported" error exit
	ret


; di should contain pointer to an indiviudual entry
print_mem_entry:
    push si
    push bp
    push bx
    push eax

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
    ; jmp $

    pop eax
    pop bx
    pop bp
    pop si

    ret

next_code:
    ; in al, 0x92
    ; test al, 2
    ; jnz .after
    ; or al, 2
    ; and al, 0xFE
    ; out 0x92, al
; .after:
    

    call do_e820
    mov si, di
    mov di, mmap_ent
.print_loop
    call print_mem_entry
    add di, 24
    cmp di, si
    jl .print_loop

    jmp $



    ; move word [reg16], [mem_type]
    ; mov eax, [mem_type]
    ; mov dword [reg32], eax
    ; call printreg32
    ; call printreg16


    jmp $



times (512*5) db 0
