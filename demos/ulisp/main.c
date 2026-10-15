#include "uart.h"

// Arithmetic builtins for RV32I / RV32E
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

unsigned int __umodsi3(unsigned int a, unsigned int b) {
    unsigned int rem = 0;
    udivmod(a, b, &rem);
    return rem;
}

static inline int abs(int x) {
    return x < 0 ? -x : x;
}

// setjmp / longjmp for RV32E
typedef unsigned int jmp_buf[16];

__attribute__((naked)) int setjmp(jmp_buf env) {
    __asm__ volatile (
        "sw ra, 0(a0)\n"
        "sw sp, 4(a0)\n"
        "sw s0, 8(a0)\n"
        "sw s1, 12(a0)\n"
        "li a0, 0\n"
        "ret\n"
    );
}

__attribute__((naked)) void longjmp(jmp_buf env, int val) {
    __asm__ volatile (
        "lw ra, 0(a0)\n"
        "lw sp, 4(a0)\n"
        "lw s0, 8(a0)\n"
        "lw s1, 12(a0)\n"
        "mv a0, a1\n"
        "bnez a0, 1f\n"
        "li a0, 1\n"
        "1: ret\n"
    );
}

// Embed program.lisp directly from file
__asm__(
    ".section .rodata\n"
    ".globl lisp_program\n"
    "lisp_program:\n"
    ".incbin \"program.lisp\"\n"
    ".byte 10\n"
    ".byte 0\n"
);
extern const char lisp_program[];

static int lisp_input_pos = 0;

int putchar(int c) {
    uart_putc((char)c);
    return c;
}

int getchar(void) {
    char c = lisp_program[lisp_input_pos];
    if (c == '\0') {
        return -1; // EOF
    }
    lisp_input_pos++;
    return (int)(unsigned char)c;
}

static inline char *strcpy(char *dest, const char *src) {
    char *d = dest;
    while ((*d++ = *src++)) {}
    return dest;
}

static inline int strcasecmp(const char *s1, const char *s2) {
    while (*s1 && *s2) {
        char c1 = *s1++;
        char c2 = *s2++;
        if (c1 >= 'A' && c1 <= 'Z') c1 += 32;
        if (c2 >= 'A' && c2 <= 'Z') c2 += 32;
        if (c1 != c2) return (unsigned char)c1 - (unsigned char)c2;
    }
    char c1 = *s1;
    char c2 = *s2;
    if (c1 >= 'A' && c1 <= 'Z') c1 += 32;
    if (c2 >= 'A' && c2 <= 'Z') c2 += 32;
    return (unsigned char)c1 - (unsigned char)c2;
}

void exit(int status) {
    (void)status;
    register int a0_reg __asm__("a0") = 0;
    __asm__ volatile("ebreak" : : "r"(a0_reg));
    while (1) {}
}

#define EOF (-1)
#define NULL ((void*)0)

typedef unsigned int uintptr_t;
typedef _Bool bool;
typedef int boolean;
#define true 1
#define false 0

#include "ulisp/ulisp_core.c"

void _start(void) {
    setup();
    setjmp(exception);
    if (lisp_program[lisp_input_pos] != '\0') {
        repl(NULL);
    }

    register int a0_reg __asm__("a0") = 0;
    __asm__ volatile("ebreak" : : "r"(a0_reg));
    while (1) {}
}
