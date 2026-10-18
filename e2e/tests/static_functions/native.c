#include <uart.h>

typedef int(*handler)(int);

void native(handler handlers[3]) {
    int n = 5;
    n = handlers[0](n);
    n = handlers[1](n);
    n = handlers[2](n);
    uart_print_uint(n);
    uart_putc('\n');
}
