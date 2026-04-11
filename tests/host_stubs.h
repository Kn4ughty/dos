#include <stdio.h>

#include "../src/kstring.h"

static inline void host_puts(StringView s)
{
        printf("%.*s", (int)s.len, s.data);
}
#define puts host_puts
// #define putchar putchar
