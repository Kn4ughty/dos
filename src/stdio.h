#ifndef STDIO_H
#define STDIO_H

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
        // #ifdef SERIAL_OUTPUT
        serial_write_string_view(s);

        // #endif
}
static inline void putchar(char c)
{
        serial_write_char(c);
}

#endif
