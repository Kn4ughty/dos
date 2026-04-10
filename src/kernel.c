#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "print_vga.h"

/* This tutorial will only work for the 32-bit ix86 targets. */
#if !defined(__i386__) && !defined(__i486__)
#error "This tutorial needs to be compiled with a ix86-elf compiler"
#endif

void kernel_main(void) {
  /* Initialize terminal interface */
  terminal_initialize();

  /* Newline support is left as an exercise. */
  terminal_writestring("Hello, kernel World!\n");
}
