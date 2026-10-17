#include <uart.h>
#include <arith.h>
#include <misc.h>

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
            uart_print_uint(num);
            if (count == 20) {
                uart_putc('\n');
            } else {
                uart_putc(' ');
            }
        }
        num++;
    }

    exit(0);
}
