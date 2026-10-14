#include "uart.h"
#include "arith.h"

extern int is_prime(unsigned int n);

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
}

void _start(void) {
    int count = 1;
    int step = 1;
    unsigned int n = 3;

    while (count < 1000) {
        if (is_prime(n)) {
            count++;
            step++;
            if (step == 100) {
                step = 0;
                print_uint(n);
                if (count == 1000) {
                    uart_putc('\n');
                } else {
                    uart_putc(' ');
                }
            }
        }
        n += 2;
    }

    register int a0_reg __asm__("a0") = 0;
    __asm__ volatile("ebreak" : : "r"(a0_reg));
    while (1) {
    }
}
