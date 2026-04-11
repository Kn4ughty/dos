#include "host_stubs.h"

char mock_buffer[MOCK_BUF_SIZE];
int mock_ptr = 0;

void reset_mock_putchar(void)
{
        memset(mock_buffer, 0, MOCK_BUF_SIZE);
        mock_ptr = 0;
}

void mock_putchar(char c)
{
        if (mock_ptr < MOCK_BUF_SIZE - 1) {
                mock_buffer[mock_ptr++] = c;
        }
}

void mock_puts(StringView s)
{
        for (size_t i = 0; i < s.len; i++) {
                mock_putchar(s.data[i]);
        }
}

#ifdef TEST_MODE
#define mock_putchar k_putchar
#define mock_puts k_puts
#endif
