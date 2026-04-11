#pragma once
#include <string.h>

#include "../src/kstring.h"

#define MOCK_BUF_SIZE 1024
extern char mock_buffer[MOCK_BUF_SIZE];
extern int mock_ptr;

void reset_mock_putchar(void);

// void mock_putchar(char c);
//
// void mock_puts(StringView s);

#ifdef TEST_MODE
#define mock_putchar k_putchar
#define mock_puts k_puts
#endif

void k_putchar(char c);
void k_puts(StringView s);
