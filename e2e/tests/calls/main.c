#include "uart.h"
#include "arith.h"

extern int native(int sum, int cnt);

int virtual(int sum, int cnt) {
    if (cnt < 0) {
        return sum;
    }
    return native(sum + cnt, cnt - 1);
}

static void print_uint(unsigned int n) {
    char buf[16];
    int len = 0;
    if (n == 0) {
        buf[len++] = '0';
    } else {
        while (n > 0) {
            unsigned int rem = 0;
            n = udivmod(n, 10, &rem);
            buf[len++] = (char)('0' + rem);
        }
    }
    for (int i = len - 1; i >= 0; i--) {
        uart_putc(buf[i]);
    }
    uart_putc('\n');
}

void _start(void) {
    int (*volatile _v)(int, int) = virtual;
    (void)_v;

    int res = native(0, 10);
    print_uint((unsigned int)res);

    register int a0_reg __asm__("a0") = 0;
    __asm__ volatile("ebreak" : : "r"(a0_reg));
    while (1) {
    }
}
