#ifndef UART_H
#define UART_H

#ifndef UART_BASE
#define UART_BASE 0x40000000
#endif

static inline void uart_putc(char c) {
    *(volatile char *)UART_BASE = c;
}

static inline void uart_puts(const char *s) {
    while (*s) {
        uart_putc(*s++);
    }
}

static inline unsigned int divmod10(unsigned int n, unsigned int *rem) {
    unsigned int q = 0;
    unsigned int r = 0;
    for (int i = 31; i >= 0; i--) {
        r = (r << 1) | ((n >> i) & 1);
        if (r >= 10) {
            r -= 10;
            q |= (1u << i);
        }
    }
    *rem = r;
    return q;
}

static inline void uart_put_num(unsigned int n) {
    if (n == 0) {
        uart_putc('0');
        return;
    }
    char buf[16];
    int i = 0;
    while (n > 0) {
        unsigned int rem = 0;
        n = divmod10(n, &rem);
        buf[i++] = (char)('0' + rem);
    }
    for (int j = i - 1; j >= 0; j--) {
        uart_putc(buf[j]);
    }
}

static inline void uart_put_hex(unsigned int n) {
    const char hex_chars[] = "0123456789abcdef";
    uart_puts("0x");
    for (int i = 28; i >= 0; i -= 4) {
        uart_putc(hex_chars[(n >> i) & 0xf]);
    }
}

#endif // UART_H
