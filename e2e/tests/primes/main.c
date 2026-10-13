#include "uart.h"
#include "arith.h"

static void print_prime(unsigned int n, int is_last) {
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
    if (is_last) {
        uart_putc('\n');
    } else {
        uart_putc(' ');
    }
}

static int is_prime(unsigned int n) {
    if (n < 2) {
        return 0;
    }
    for (unsigned int d = 2; ; d++) {
        unsigned int rem = 0;
        unsigned int q = udivmod(n, d, &rem);
        if (d > q) {
            break;
        }
        if (rem == 0) {
            return 0;
        }
    }
    return 1;
}

void _start(void) {
    int count = 0;
    unsigned int num = 2;

    while (count < 20) {
        if (is_prime(num)) {
            count++;
            print_prime(num, count == 20);
        }
        num++;
    }

    register int a0_reg __asm__("a0") = 0;
    __asm__ volatile("ebreak" : : "r"(a0_reg));
    while (1) {
    }
}
