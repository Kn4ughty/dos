TEXT_VIDEO_MEMORY equ 0xb8000

sprint:
    lodsb
    cmp al, 0
    jz short .done
    call cprint
    jmp short sprint
.done:
    ret

println:
    call sprint
    add byte [ypos], 1  ; down one row
    mov byte [xpos], 0  ; back to left of screen 
    ret

; prints character at al
cprint:
    cmp al, 10
    jne short .draw_char
    add byte [ypos], 1
    mov byte [xpos], 0
    ret
.draw_char:
    mov ah, 0x0F ; attrib = white on black
    mov ecx, eax

    movzx eax, byte [ypos]

    mov edx, 160 ; total length of bytes (80cols * stride of 2)
    mul edx ; y pos * 160.

    movzx ebx, byte [xpos]
    shl ebx, 1 ; bx = x * 2

    mov edi, TEXT_VIDEO_MEMORY
    add edi, eax
    add edi, ebx

    mov eax, ecx
    mov word [ds:edi], ax
    add byte [xpos], 1

    ret

;todo. Make printreg8 function
; maybe make new byte -> 2char procedure?


; write contents you want printed to reg16
; i.e mov word [reg16], ax
printreg16:
    push di
    push ax
    push si
    push cx
    push bx

    mov di, outstr16 ; di = &string
    mov ax, [reg16] ; ax = *reg
    mov si, hexstr  ; si = &hexstr
    mov cx, 4 ; four places?
.hexloop:
    ; rotate the bits in ax, 4 times.
    ; i.e, shift left by 1 nibble
    rol ax, 4
    mov bx, ax
    and bx, 0x000F ; get original rightmost nibble

    ; how does it calculate si +bx if its different at runtime?
    mov bl, [si + bx] ; bl = *(&hexstr + character)
    mov [di], bl  ; *id = bl 
    inc di
    dec cx
    jnz short .hexloop

    mov si, outstr16
    call sprint
    mov al, ' '
    call cprint

    pop bx
    pop cx
    pop si
    pop ax
    pop di

    ret


; write contents you want printed to reg32
; i.e mov dword [reg32], eax
printreg32:
    push si
    push edi
    push eax
    push ecx
    push ebx

    mov edi, outstr32
    mov eax, [reg32]
    mov si, hexstr
    mov ecx, 8
.hexloop:
    ; rotate the bits in ax, 4 times.
    ; i.e, shift left by 1 nibble
    rol eax, 4
    mov ebx, eax
    and ebx, 0x0F ; get original rightmost nibble

    ; how does it calculate si +bx if its different at runtime?
    mov bl, [esi + ebx] ; bl = *(&hexstr + character)
    mov byte [edi], bl  ; *id = bl 
    inc edi
    dec ecx
    jnz short .hexloop

    mov si, outstr32
    call sprint

    pop ebx
    pop ecx
    pop eax
    pop edi
    pop si

    ret

xpos db 0
ypos db 0

hexstr db "0123456789ABCDEF"
decstr db "0123456789"

outstr16   db  "0000", 0  ;register value string
reg16 dw 0

outstr32   db "00000000", 0  ;register value string
reg32 dd 0

