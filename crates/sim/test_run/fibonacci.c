#include "uart.h"

static int fib(int n) {
    if (n <= 0) return 0;
    if (n == 1) return 1;
    return fib(n - 1) + fib(n - 2);
}

int main(void) {
    uart_puts("Fibonacci test:\n");
    for (int i = 0; i <= 10; i++) {
        uart_put_num(i);
        uart_puts(": ");
        uart_put_num(fib(i));
        uart_putc('\n');
    }
    uart_puts("Fibonacci done.\n");
    return 0;
}
