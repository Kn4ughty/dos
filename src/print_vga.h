#ifndef PRINT_VGA_H
#define PRINT_VGA_H

#include <stdint.h>

void terminal_initialize(void);
void terminal_writestring(const char *data);
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

#endif
