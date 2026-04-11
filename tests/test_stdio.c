#include <stdio.h>
#include <string.h>

#include "greatest.h"

#include "../src/kstdio.h"

TEST test_atoi_basic()
{
        ASSERT_EQ_FMT(123, k_atoi(sv("123")), "%d");
        PASS();
}

SUITE(string_logic)
{
        RUN_TEST(test_atoi_basic);
}

GREATEST_MAIN_DEFS();

int main(int argc, char **argv)
{
        GREATEST_MAIN_BEGIN();
        RUN_SUITE(string_logic);
        GREATEST_MAIN_END();
}
