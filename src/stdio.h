#pragma once

#define VGA_OUTPUT

#include "print_vga.h"
#include "serial.h"
#include "string.h"

// Trying to copy https://en.cppreference.com/w/c/header/stdio
// see also https://www.w3schools.com/c/c_ref_stdio.php

/*
 *
 */
static inline void puts(StringView s)
{
#if defined(SERIAL_OUTPUT)
        serial_write_string_view(s);
#elif defined(VGA_OUTPUT)
        vga_write_string_view(s);
#else
#error "You need to define an output method!"
#endif
}
static inline void putchar(char c)
{
#if defined(SERIAL_OUTPUT)
        serial_write_char(c);
#elif defined(VGA_OUTPUT)
        vga_putchar(c);
#else
#error "You need to define an output method!"
#endif
}

#define printf printf_
int printf_(StringView format, ...);
