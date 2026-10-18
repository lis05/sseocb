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

handler get_dir_a(void);
void native(handler[3]);

void _start(void) {
    handler handlers[3] = {a, b, get_dir_a()};
    native(handlers);
    exit(0);
}
