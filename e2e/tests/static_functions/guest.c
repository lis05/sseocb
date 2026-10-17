#include <uart.h>
#include <arith.h>
#include <misc.h>

static int a(int n) {
    return n + 5;
}

static int b(int n) {
    return n + 10;
}

typedef int (*handler)(int);

void native(handler[2]);

void _start(void) {
    handler handlers[2] = {a, b};
    native(handlers);
    exit(0);
}
