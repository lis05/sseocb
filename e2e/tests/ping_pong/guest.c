#include <uart.h>
#include <misc.h>

int pong(int n);

int ping(int n) {
    if (n <= 0) return 0;
    return pong(n - 1) + n;
}

void _start(void) {
    // Test to see if vP -> native -> vP -> native -> ... chains function properly.
    // Each int stack frame is about 60-80 bytes, so running it 50 times with 8KiB
    // stack is fine.

    int n = 50;
    uart_print_uint(ping(50));
    uart_putc('\n');
    exit(0);
}
