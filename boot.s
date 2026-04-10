/* Declare constants for the multiboot header. */
# https://www.gnu.org/software/grub/manual/multiboot/multiboot.html
.set ALIGN,    1<<0             /* align loaded modules on page boundaries */
.set MEMINFO,  1<<1             /* provide memory map */
.set VIDINFO,  0<<2             /* provide video mode */
.set FLAGS,    ALIGN | MEMINFO | VIDINFO
.set MAGIC,    0x1BADB002       /* 'magic number' lets bootloader find the header */
.set CHECKSUM, -(MAGIC + FLAGS) /* checksum of above, to prove we are multiboot */

/* 
Declare a multiboot header that marks the program as a kernel. These are magic
values that are documented in the multiboot standard. The bootloader will
search for this signature in the first 8 KiB of the kernel file, aligned at a
32-bit boundary. The signature is in its own section so the header can be
forced to be within the first 8 KiB of the kernel file.
*/
.section .multiboot
.align 4
.long MAGIC
.long FLAGS
.long CHECKSUM

/*
The multiboot standard does not define the value of the stack pointer register
(esp) and it is up to the kernel to provide a stack. This allocates room for a
small stack by creating a symbol at the bottom of it, then allocating 16384
bytes for it, and finally creating a symbol at the top. The stack grows
downwards on x86. The stack is in its own section so it can be marked nobits,
which means the kernel file is smaller because it does not contain an
uninitialized stack. The stack on x86 must be 16-byte aligned according to the
System V ABI standard and de-facto extensions. The compiler will assume the
stack is properly aligned and failure to align the stack will result in
undefined behavior.
*/
.section .bss
.align 16
stack_bottom:
.skip 16384 # 16 KiB
stack_top:

/*
The linker script specifies _start as the entry point to the kernel and the
bootloader will jump to this position once the kernel has been loaded. It
doesn't make sense to return from this function as the bootloader is gone.
*/
.section .text
.global _start
.type _start, @function
_start:

    cmp $0x2BADB002, %eax
    jne not_multiboot

    # move src, dest
    # mov address of stack top into esp register
    # Address=base+(index×scale)+disp
	mov $stack_top, %esp

	/*
	This is a good place to initialize crucial processor state before the
	high-level kernel is entered. It's best to minimize the early
	environment where crucial features are offline. Note that the
	processor is not fully initialized yet: Features such as floating
	point instructions and instruction set extensions are not initialized
	yet. The GDT should be loaded here. Paging should be enabled here.
	C++ features such as global constructors and exceptions will require
	runtime support to work as well.
	*/

	/*
	Enter the high-level kernel. The ABI requires the stack is 16-byte
	aligned at the time of the call instruction (which afterwards pushes
	the return pointer of size 4 bytes). The stack was originally 16-byte
	aligned above and we've pushed a multiple of 16 bytes to the
	stack since (pushed 0 bytes so far), so the alignment has thus been
	preserved and the call is well defined.
	*/
	call kernel_main # how does the linker know?

	/*
	If the system has nothing more to do, put the computer into an
	infinite loop. To do that:
	1) Disable interrupts with cli (clear interrupt enable in eflags).
	   They are already disabled by the bootloader, so this is not needed.
	   Mind that you might later enable interrupts and return from
	   kernel_main (which is sort of nonsensical to do).
	2) Wait for the next interrupt to arrive with hlt (halt instruction).
	   Since they are disabled, this will lock up the computer.
	3) Jump to the hlt instruction if it ever wakes up due to a
	   non-maskable interrupt occurring or due to system management mode.
	*/
	cli
1:	hlt
	jmp 1b



/*
Set the size of the _start symbol to the current location '.' minus its start.
This is useful when debugging or when you implement call tracing.
*/
.size _start, . - _start

TXT_not_multiboot: .asciz "Was not booted from multiboot!"

not_multiboot:
    mov TXT_not_multiboot, %esi
    call sprint

.set TEXT_VIDEO_MEMORY, 0xb8000
xpos: .byte 0
ypos: .byte 0

# set %esi
sprint:
    lodsb
    cmpb $0, %al
    jz .done
    call cprint
    jmp sprint
.done:
    ret

println:
    call sprint
    incb ypos
    movb $0, xpos  # back to left of screen 
    ret

# prints character at al
cprint:
    pushl %ebx
    pushl %ecx
    pushl %eax
    pushl %edx
    pushl %edi

    cmpb $10, %al
    jne .draw_char
    incb ypos
    movb $0, xpos  # back to left of screen 
    jmp .exit
.draw_char:
    mov 0x0F, %ah # attrib = white on black
    mov %eax, %ecx

    movzbl ypos, %eax

    movl $160, %edx # total length of bytes (80cols * stride of 2)
    mull %edx # y pos * 160.

    movzbl xpos, %ebx
    shll $1, %ebx # bx = x * 2

    movl $TEXT_VIDEO_MEMORY, %edi
    addl %eax, %edi
    addl %ebx, %edi
    
    movw %cx, (%edi)
    incb xpos

.exit:
    pop %edi
    pop %edx
    pop %eax
    pop %ecx
    pop %ebx
    ret
