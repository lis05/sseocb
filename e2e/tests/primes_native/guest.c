#include <uart.h>
#include <misc.h>

int is_prime(unsigned int n);

void _start(void) {
    int count = 0;
    unsigned int num = 2;

    while (count < 20) {
        if (is_prime(num)) {
            count++;
            uart_print_uint(num);
            if (count == 20) {
                uart_putc('\n');
            } else {
                uart_putc(' ');
            }
        }
        num++;
    }

    exit(0);
}
