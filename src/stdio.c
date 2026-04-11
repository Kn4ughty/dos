
#include <stdint.h>

#include "stdio.h"

uint32_t rol(uint32_t value, uint32_t count)
{
        return (value << count) | (value >> (32 - count));
}

void printhex(uint32_t input)
{
        const char *hexstr = "0123456789ABCDEF";
        char outstr[] = "00000000";
        for (int i = 0; i < 8; i++) {
                int idx =
                    rol(input, 4 * (i + 1)) & 0x0F; // get new rightmost nibble
                char c = hexstr[idx];
                outstr[i] = c;
        }
        puts(sv(outstr));
}

void printbin8(uint8_t input)
{
        const char *binstr = "01";
        char outstr[8];
        for (int i = 0; i < 8; i++) {
                int idx = (input >> i) & 1;
                char c = binstr[idx];
                outstr[7 - i] = c;
        }
        puts(sv(outstr));
}

void printbin16(uint16_t input)
{
        const char *binstr = "01";
        char outstr[16];
        for (int i = 0; i < 16; i++) {
                int idx = (input >> i) & 1;
                char c = binstr[idx];
                outstr[15 - i] = c;
        }
        puts(sv(outstr));
}

void printbin32(uint32_t input)
{
        const char *binstr = "01";
        char outstr[32];
        for (int i = 0; i < 32; i++) {
                int idx = (input >> i) & 1;
                char c = binstr[idx];
                outstr[31 - i] = c;
        }
        puts(sv(outstr));
}
