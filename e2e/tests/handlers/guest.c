#include <uart.h>
#include <misc.h>

int virtual_handler(int n) {
    return n * 2;
}

int native_handler(int n);

void _start(void) {
    // Here, virtual_handler is not used in guest code, but a trampoline for it must
    // be produced nonetheless as it is used by the native code.
    int n = 10;
    uart_print_uint(native_handler(n));
    uart_putc('\n');
    exit(0);
}
