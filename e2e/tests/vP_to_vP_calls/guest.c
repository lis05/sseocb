#include <uart.h>
#include <arith.h>
#include <misc.h>

int fib(int n) {
    if (n <= 1) return n;
    return fib(n - 1) + fib(n - 2);
}

// To force sseocb to make a trampoline for fib
volatile void* ptr = (void*)&fib;

void _start(void) {
    // The idea behind this test is to check that vP -> vP calls function normally
    // when the function is exposed via a trampoline.
    //
    // If vP -> vP looks like vP -> trampoline -> vP, each nesting level of fib will
    // create a huge (60B+) stack frame on int's stack, which will fail as the
    // interpreter only has 128 bytes of stack in this example.
    uart_print_uint(fib(10));
    uart_putc('\n');

    exit(0);
}
