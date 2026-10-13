#include "uart.h"
#include "arith.h"

typedef int (*op_handler_t)(int, int);

enum OpType {
    OP_PLUS = 0,
    OP_MINUS = 1,
    OP_MULT = 2,
    OP_DIV = 3,
    NUM_OPS = 4
};

static op_handler_t handlers[NUM_OPS];

static void install_handler(enum OpType op, op_handler_t fn) {
    handlers[op] = fn;
}

static int op_plus(int a, int b) {
    return a + b;
}

static int op_minus(int a, int b) {
    return a - b;
}

static int op_mult(int a, int b) {
    int res = 0;
    int neg = 0;
    if (b < 0) {
        b = -b;
        neg = !neg;
    }
    while (b > 0) {
        if (b & 1) {
            res += a;
        }
        a <<= 1;
        b >>= 1;
    }
    return neg ? -res : res;
}

static int op_div(int a, int b) {
    if (b == 0) {
        return 0;
    }
    int neg = 0;
    if (a < 0) {
        a = -a;
        neg = !neg;
    }
    if (b < 0) {
        b = -b;
        neg = !neg;
    }
    unsigned int q = udivmod((unsigned int)a, (unsigned int)b, 0);
    return neg ? -(int)q : (int)q;
}

struct Operation {
    enum OpType op;
    int a;
    int b;
};

static void print_int(int n) {
    if (n < 0) {
        uart_putc('-');
        n = -n;
    }
    char buf[16];
    int len = 0;
    if (n == 0) {
        buf[len++] = '0';
    } else {
        unsigned int un = (unsigned int)n;
        while (un > 0) {
            unsigned int rem = 0;
            un = udivmod(un, 10, &rem);
            buf[len++] = (char)('0' + rem);
        }
    }
    for (int i = len - 1; i >= 0; i--) {
        uart_putc(buf[i]);
    }
}

void _start(void) {
    install_handler(OP_PLUS, op_plus);
    install_handler(OP_MINUS, op_minus);
    install_handler(OP_MULT, op_mult);
    install_handler(OP_DIV, op_div);

    static const struct Operation ops[] = {
        { OP_PLUS,  10, 20 },
        { OP_MINUS, 50, 8 },
        { OP_MULT,  6, 7 },
        { OP_DIV,   100, 4 },
        { OP_PLUS,  -5, 15 },
        { OP_MINUS, 10, -5 },
        { OP_MULT,  -3, 4 },
        { OP_DIV,   81, 9 }
    };

    int n_ops = sizeof(ops) / sizeof(ops[0]);
    for (int i = 0; i < n_ops; i++) {
        int res = handlers[ops[i].op](ops[i].a, ops[i].b);
        print_int(res);
        if (i == n_ops - 1) {
            uart_putc('\n');
        } else {
            uart_putc(' ');
        }
    }

    register int a0_reg __asm__("a0") = 0;
    __asm__ volatile("ebreak" : : "r"(a0_reg));
    while (1) {
    }
}
