#ifndef DEMOS_UART_H
#define DEMOS_UART_H

#define UART_BASE 0x40000000

static inline void uart_putc(char c) {
    *(volatile char *)UART_BASE = c;
}

static inline void uart_puts(const char *s) {
    while (*s) {
        uart_putc(*s++);
    }
}

#endif
