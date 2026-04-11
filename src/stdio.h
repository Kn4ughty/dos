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

void printhex(uint32_t input);

void printbin8(uint8_t input);
void printbin16(uint16_t input);
void printbin32(uint32_t input);
#define printbin(T)                                                            \
        _Generic((T),                                                          \
            uint8_t: printbin8,                                                \
            int8_t: printbin8,                                                 \
            uint16_t: printbin16,                                              \
            int16_t: printbin16,                                               \
            uint32_t: printbin32,                                              \
            int32_t: printbin32)(T)
