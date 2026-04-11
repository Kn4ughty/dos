// #include <stdio.h>
#include <string.h>

#include "greatest.h"

#include "../src/kstdio.h"

#include "host_stubs.h"

TEST test_atoi_basic()
{
        ASSERT_EQ_FMT(123, k_atoi(sv("123")), "%d");
        ASSERT_EQ_FMT(607, k_atoi(sv("0607")), "%d");
        PASS();
}

TEST test_printf()
{
        reset_mock_putchar();
        k_printf(sv("%%"));
        ASSERT_STR_EQ("%", mock_buffer);

        reset_mock_putchar();
        k_printf(sv("c%c"), 'h');
        ASSERT_STR_EQ("ch", mock_buffer);

        // Number display
        reset_mock_putchar();
        k_printf(sv("t: %d"), 123);
        ASSERT_STR_EQ("t: 123", mock_buffer);

        reset_mock_putchar();
        k_printf(sv("hex: %X"), 0xF123);
        ASSERT_STR_EQ("hex: F123", mock_buffer);

        reset_mock_putchar();
        k_printf(sv("bin: %b"), 0b110101);
        ASSERT_STR_EQ("bin: 110101", mock_buffer);

        PASS();
}

SUITE(string_logic)
{
        RUN_TEST(test_atoi_basic);
        RUN_TEST(test_printf);
}

GREATEST_MAIN_DEFS();

int main(int argc, char **argv)
{
        GREATEST_MAIN_BEGIN();
        RUN_SUITE(string_logic);
        GREATEST_MAIN_END();
}
