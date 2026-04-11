
#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>

#include "kstdio.h"

uint32_t rol(uint32_t value, uint32_t count)
{
        return (value << count) | (value >> (32 - count));
}

#define isdigit isdigit_
inline bool isdigit_(char c)
{
        return (c >= '0' && c <= '9');
}

// int printbasen(uint32_t input, uint8_t base, int padding)
int printbasen(uint32_t input, uint8_t base)
{
        if (base > 16 || base < 2) {
                // cannot print base larger than 16 (for now)!
                // base 1 is also not supported lol (tally marks)
                return -1;
        }
        StringView base_str = sv("0123456789ABCDEF");
        base_str.len = base;

        // walk backwards so reversing isnt needed, and to easily add padding
        char outstr[32]; // max possible (base 2)
        int i = 31;
        while (input) {
                outstr[i] = base_str.data[input % base];
                input /= base;
                i--;
        }

        k_puts((StringView){.data = outstr + i + 1, .len = 31 - i});
        return 0;
}

/*
 * # Printing Hex
 * `printf("%#05x\n", 0x1f)` -> 0x01f
 * > print in hex (x), with format specfier (#) with length of 5 characters
 * with 0 prefixing.
 * I will support either 0 prefixing or space prefixing.
 * space prefixing spaces out the entire thing. output would become:
 * `printf("%#05x\n", 0x1f)` -> " 0x1f"
 *
 */
int k_printf(StringView format, ...)
{
        va_list args;
        va_start(args, format);
        // char buffer[1];
        //

        size_t i = 0;
        while (i < format.len) {
                char c = format.data[i++];

                if (c != '%') {
                        k_putchar(c);
                        continue;
                }

                if (i >= format.len)
                        break;

                char specfier = format.data[i++];
                if (isdigit(specfier)) {
                }

                switch (specfier) {
                case 'c':
                        k_putchar((char)va_arg(args, int));
                        break;
                case 'X':
                case 'x':
                        printbasen(va_arg(args, uint32_t), 16);
                        break;
                case 'd':
                        printbasen(va_arg(args, uint32_t), 10);
                        break;
                case 'b':
                        printbasen(va_arg(args, uint32_t), 2);
                        break;
                case '%':
                        k_putchar('%');
                        break;
                default:
                        // unknown specfier, print it out literally
                        k_putchar('%');
                        k_putchar(specfier);
                        break;
                }
        }

        va_end(args);

        return 0;
}

/* returns 0 if conversion fails
 * input must be base 10
 * assumes input is already trimmed to be just the number part
 */
int k_atoi(StringView input)
{
        int output = 0;
        for (size_t i = 0; i < input.len; i++) {
                char c = input.data[i];
                if (!isdigit(c)) {
                        return 0;
                }
                int digit = c - '0';
                output = (output * 10) + digit;
        }

        return output;
}
