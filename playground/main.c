#define UART_BASE 0x40000000

static inline void uart_putc(char c) {
    *(volatile char *)UART_BASE = c;
}

static inline void uart_puts(const char *s) {
    while (*s) {
        uart_putc(*s++);
    }
}

/* Freestanding compiler-rt builtins */
int __mulsi3(int a, int b) {
    int res = 0;
    int neg = 0;
    if (a < 0) { a = -a; neg = !neg; }
    if (b < 0) { b = -b; neg = !neg; }
    while (b > 0) {
        if (b & 1) res += a;
        a <<= 1;
        b >>= 1;
    }
    return neg ? -res : res;
}

int __divsi3(int a, int b) {
    if (b == 0) return 0;
    int neg = 0;
    if (a < 0) { a = -a; neg = !neg; }
    if (b < 0) { b = -b; neg = !neg; }
    int q = 0;
    for (int i = 30; i >= 0; i--) {
        if ((a >> i) >= b) {
            a -= (b << i);
            q |= (1 << i);
        }
    }
    return neg ? -q : q;
}

int __modsi3(int a, int b) {
    if (b == 0) return 0;
    return a - __mulsi3(__divsi3(a, b), b);
}

void *memcpy(void *dest, const void *src, unsigned int n) {
    char *d = (char *)dest;
    const char *s = (const char *)src;
    while (n--) *d++ = *s++;
    return dest;
}

void *memset(void *dest, int c, unsigned int n) {
    char *d = (char *)dest;
    while (n--) *d++ = (char)c;
    return dest;
}

int printf(const char *fmt, ...) {
    uart_puts(fmt);
    return 0;
}

static void uart_put_num(int n) {
    if (n == 0) {
        uart_putc('0');
        return;
    }
    if (n < 0) {
        uart_putc('-');
        n = -n;
    }
    char buf[16];
    int i = 0;
    while (n > 0) {
        buf[i++] = (char)('0' + (n % 10));
        n /= 10;
    }
    for (int j = i - 1; j >= 0; j--) {
        uart_putc(buf[j]);
    }
}

int is_prime(int n) {
    if (n < 2) return 0;
    for (int d = 2; d * d <= n; d++) {
        if (n % d == 0) return 0;
    }
    return 1;
}

volatile int g_primes[100];

void _start(void) {
    uart_puts("Calculating first 100 primes:\n");

    int count = 0;
    int candidate = 2;

    while (count < 100) {
        if (is_prime(candidate)) {
            g_primes[count] = candidate;
            uart_put_num(candidate);
            count++;
            if (count % 10 == 0) {
                uart_putc('\n');
            } else {
                uart_putc(' ');
            }
        }
        candidate++;
    }

    uart_puts("Done calculating 100 primes.\n");

    while (1) {
    }
}

int main(void) {
    _start();
    return 0;
}
