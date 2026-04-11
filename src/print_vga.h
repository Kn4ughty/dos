#pragma once

#include <stdint.h>

#include "kstring.h"

void terminal_initialize(void);
void vga_write_string_view(StringView sv);
void vga_putchar(char c);
