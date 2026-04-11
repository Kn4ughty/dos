#pragma once

#include <stddef.h>

static inline size_t strlen(const char *str)
{
        size_t len = 0;
        while (str[len])
                len++;
        return len;
}

typedef struct {
        const char *data;
        size_t len;
} StringView;
#define SV_LIT(S) (StringView){.data = S, .len = sizeof(S) - 1}

static inline StringView sv(const char *cstr)
{
        return (StringView){.data = cstr, .len = strlen(cstr)};
}
