#include "native.h"

unsigned int udivmod(unsigned int n, unsigned int d, unsigned int *rem) {
    unsigned int q = 0;
    unsigned int r = 0;
    for (int i = 31; i >= 0; i--) {
        r = (r << 1) | ((n >> i) & 1);
        if (r >= d) {
            r -= d;
            q |= (1u << i);
        }
    }
    if (rem) {
        *rem = r;
    }
    return q;
}

int is_prime(unsigned int n) {
    if (n < 2) return 0;
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

volatile void *ptr1 = (void*)&udivmod;

void _start(void) {
    __asm__(".globl __before_print");
    __asm__("__before_print:");
    uart_puts("--- First 10 Primes ---\n");
    __asm__(".globl __after_print");
    __asm__("__after_print:");

    int count = 0;
    unsigned int num = 2;
    while (count < 10) {
        if (is_prime(num)) {
            native_print_uint(num);
            count++;
            if (count < 10) {
                uart_puts(" ");
            } else {
                uart_puts("\n");
            }
        }
        num++;
    }

    register int a0_reg __asm__("a0") = 0;
    __asm__ volatile("ebreak" : : "r"(a0_reg));
    while (1) {
    }
}
