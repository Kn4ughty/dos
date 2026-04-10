
#ifndef SERIAL_H
#define SERIAL_H

#include "string.h"

bool init_serial();
void serial_write_char(char a);
void serial_write_string_view(StringView s);

#endif
