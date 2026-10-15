#include "uart.h"
#include "ubasic/ubasic.h"

// Arithmetic builtins for RV32I
unsigned int __mulsi3(unsigned int a, unsigned int b) {
    unsigned int res = 0;
    while (b > 0) {
        if (b & 1) res += a;
        a <<= 1;
        b >>= 1;
    }
    return res;
}

static unsigned int udivmod(unsigned int n, unsigned int d, unsigned int *rem) {
    if (d == 0) {
        if (rem) *rem = 0;
        return 0;
    }
    unsigned int q = 0, r = 0;
    for (int i = 31; i >= 0; i--) {
        r = (r << 1) | ((n >> i) & 1);
        if (r >= d) {
            r -= d;
            q |= (1u << i);
        }
    }
    if (rem) *rem = r;
    return q;
}

unsigned int __udivsi3(unsigned int n, unsigned int d) {
    return udivmod(n, d, 0);
}

int __divsi3(int a, int b) {
    int neg = 0;
    if (a < 0) { a = -a; neg = !neg; }
    if (b < 0) { b = -b; neg = !neg; }
    int res = (int)udivmod((unsigned int)a, (unsigned int)b, 0);
    return neg ? -res : res;
}

int __modsi3(int a, int b) {
    int neg = (a < 0);
    if (a < 0) a = -a;
    if (b < 0) b = -b;
    unsigned int rem = 0;
    udivmod((unsigned int)a, (unsigned int)b, &rem);
    return neg ? -(int)rem : (int)rem;
}

// Memory pool for line_index (replaces dynamic malloc)
#define POOL_SIZE 64
static char mem_pool[POOL_SIZE * 16];
static int pool_offset = 0;

void *malloc(unsigned int size) {
    size = (size + 3) & ~3;
    if (pool_offset + size <= sizeof(mem_pool)) {
        void *p = &mem_pool[pool_offset];
        pool_offset += size;
        return p;
    }
    return (void *)0;
}

void free(void *ptr) {
    (void)ptr;
}

void exit(int status) {
    (void)status;
    register int a0_reg __asm__("a0") = 0;
    __asm__ volatile("ebreak" : : "r"(a0_reg));
    while (1) {}
}

int printf(const char *fmt, ...) {
    __builtin_va_list args;
    __builtin_va_start(args, fmt);
    while (*fmt) {
        if (*fmt == '%') {
            fmt++;
            if (*fmt == 's') {
                const char *s = __builtin_va_arg(args, const char *);
                if (!s) s = "(null)";
                while (*s) uart_putc(*s++);
            } else if (*fmt == 'd' || *fmt == 'i') {
                int d = __builtin_va_arg(args, int);
                if (d < 0) {
                    uart_putc('-');
                    d = -d;
                }
                char buf[12];
                int len = 0;
                if (d == 0) {
                    buf[len++] = '0';
                } else {
                    while (d > 0) {
                        unsigned int rem = 0;
                        d = (int)udivmod((unsigned int)d, 10, &rem);
                        buf[len++] = (char)('0' + rem);
                    }
                }
                for (int i = len - 1; i >= 0; i--) {
                    uart_putc(buf[i]);
                }
            } else if (*fmt == 'c') {
                char c = (char)__builtin_va_arg(args, int);
                uart_putc(c);
            }
        } else {
            uart_putc(*fmt);
        }
        fmt++;
    }
    __builtin_va_end(args);
    return 0;
}

static const char basic_program[] =
    "10 print \"#   ##   #\"\n"
    "20 print \" #  ##  # \"\n"
    "30 print \"  # ## #  \"\n"
    "40 print \"#  ####  #\"\n"
    "50 print \"##########\"\n"
    "60 print \"##########\"\n"
    "70 print \"#  ####  #\"\n"
    "80 print \"  # ## #  \"\n"
    "90 print \" #  ##  # \"\n"
    "100 print \"#   ##   #\"\n"
    "110 end\n";

void _start(void) {
    ubasic_init(basic_program);

    do {
        ubasic_run();
    } while (!ubasic_finished());

    register int a0_reg __asm__("a0") = 0;
    __asm__ volatile("ebreak" : : "r"(a0_reg));
    while (1) {}
}

#include "ubasic/tokenizer.c"
#include "ubasic/ubasic.c"
