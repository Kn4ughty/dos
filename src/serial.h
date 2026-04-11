#pragma once

#include <stdbool.h>

#include "kstdio.h"
#include "kstring.h"

bool init_serial();
void serial_write_char(char a);
void serial_write_string_view(StringView s);
