#include "native.h"

#define UART_BASE 0x40000000

void uart_putc(char c) {
    *(volatile char *)UART_BASE = c;
}

void uart_puts(const char *s) {
    while (*s) {
        uart_putc(*s++);
    }
}

static unsigned int udiv10(unsigned int n, unsigned int *rem) {
    unsigned int q = 0;
    unsigned int r = 0;
    for (int i = 31; i >= 0; i--) {
        r = (r << 1) | ((n >> i) & 1);
        if (r >= 10) {
            r -= 10;
            q |= (1u << i);
        }
    }
    if (rem) {
        *rem = r;
    }
    return q;
}

void native_print_uint(unsigned int n) {
    char buf[16];
    int len = 0;
    if (n == 0) {
        buf[len++] = '0';
    } else {
        while (n > 0) {
            unsigned int rem = 0;
            n = udiv10(n, &rem);
            buf[len++] = (char)('0' + rem);
        }
    }
    for (int i = len - 1; i >= 0; i--) {
        uart_putc(buf[i]);
    }
}
