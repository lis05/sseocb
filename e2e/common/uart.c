#include "uart.h"
#include "arith.h"

#define UART_BASE 0x40000000

void uart_putc(char c) {
    *(volatile char *)UART_BASE = c;
}

void uart_puts(const char *s) {
    while (*s) {
        uart_putc(*s++);
    }
}

void uart_print_uint(unsigned int n) {
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

void uart_print_int(int n) {
    if (n < 0) {
        uart_putc('-');
        uart_print_uint((unsigned int)(-(n + 1)) + 1u);
    } else {
        uart_print_uint((unsigned int)n);
    }
}
