#pragma once

#include "kstring.h"

#ifdef TEST_MODE
#include "../tests/host_stubs.h"
#else
#define VGA_OUTPUT
#include "print_vga.h"
#include "serial.h"

// Trying to copy https://en.cppreference.com/w/c/header/stdio
// see also https://www.w3schools.com/c/c_ref_stdio.php

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

#endif

// #define printf printf_
int k_printf(StringView format, ...);

int k_atoi(StringView input);

#ifndef TEST_MODE
#define printf k_printf
#define atoi k_atoi
#endif
