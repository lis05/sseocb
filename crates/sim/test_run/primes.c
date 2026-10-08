#include "uart.h"

// In .data: must be copied from Flash to RAM by CRT0
static int max_limit = 30;
// In .bss: must be zeroed in RAM by CRT0
static int zero_check;

static int is_prime(int n) {
    if (n < 2) return 0;
    for (int d = 2; d < n; d++) {
        int r = n;
        while (r >= d) {
            r -= d;
        }
        if (r == 0) return 0;
    }
    return 1;
}

int main(void) {
    // Assert .data and .bss initialization
    if (max_limit != 30 || zero_check != 0) {
        uart_puts("DATA/BSS initialization failed!\n");
        return 1;
    }

    uart_puts("Primes test:\n");
    int prime_count = 0;
    for (int i = 2; i <= max_limit; i++) {
        if (is_prime(i)) {
            if (prime_count > 0) {
                uart_putc(' ');
            }
            uart_put_num(i);
            prime_count++;
        }
    }
    uart_putc('\n');
    uart_puts("Total primes: ");
    uart_put_num(prime_count);
    uart_putc('\n');
    uart_puts("Primes done.\n");
    return 0;
}
