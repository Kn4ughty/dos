#pragma once

#include <stddef.h>

static inline size_t kstrlen(const char *str)
{
        size_t len = 0;
        while (str[len])
                len++;
        return len;
}

#ifndef TEST_MODE
#define strlen kstrlen
#endif

typedef struct {
        const char *data;
        size_t len;
} StringView;
// Use only for string literals.
#define SV(S) (StringView){.data = S, .len = sizeof(S) - 1}

static inline StringView sv(const char *cstr)
{
        return (StringView){.data = cstr, .len = kstrlen(cstr)};
}
