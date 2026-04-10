#ifndef STDIO_H
#define STDIO_H

#include "string.h"

// Trying to copy https://en.cppreference.com/w/c/header/stdio
// see also https://www.w3schools.com/c/c_ref_stdio.php



bool init_serial();
void serial_write_char(char a);
void serial_write_string_view(StringView s);

#ifdef SERIAL_OUTPUT
static inline void puts(StringView s) {
    serial_write_string_view(s);
}
static inline void putchar(char c) { 
    serial_write_char(c);  
}
#endif

#endif
