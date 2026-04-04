sprint:
    lodsb
    cmp al, 0
    jz .done
    call cprint
    jmp sprint
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


printreg16:
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
    jnz .hexloop

    mov si, outstr16
    call sprint

    ret


xpos db 0
ypos db 0

hexstr db "0123456789ABCDEF"
outstr16   db "0000", 0  ;register value string
reg16 dw 0

