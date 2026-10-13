#define UART_BASE 0x40000000

static inline void uart_putc(char c) {
    *(volatile char *)UART_BASE = c;
}

static inline void uart_puts(const char *s) {
    while (*s) {
        uart_putc(*s++);
    }
}

// Software division and modulo for RV32I / RV32E (no hardware M extension)
static unsigned int udivmod(unsigned int n, unsigned int d, unsigned int *rem) {
    unsigned int q = 0;
    unsigned int r = 0;
    for (int i = 31; i >= 0; i--) {
        r = (r << 1) | ((n >> i) & 1);
        if (r >= d) {
            r -= d;
            q |= (1u << i);
        }
    }
    if (rem) {
        *rem = r;
    }
    return q;
}

static int is_prime(unsigned int n) {
    if (n < 2) return 0;
    for (unsigned int d = 2; ; d++) {
        unsigned int rem = 0;
        unsigned int q = udivmod(n, d, &rem);
        if (d > q) {
            break; // d * d > n
        }
        if (rem == 0) {
            return 0;
        }
    }
    return 1;
}

static void print_uint_padded(unsigned int n, int width) {
    char buf[16];
    int len = 0;
    if (n == 0) {
        buf[len++] = '0';
    } else {
        while (n > 0) {
            unsigned int rem = 0;
            n = udivmod(n, 10, &rem);
            buf[len++] = (char)('0' + rem);
        }
    }
    for (int i = len; i < width; i++) {
        uart_putc(' ');
    }
    for (int i = len - 1; i >= 0; i--) {
        uart_putc(buf[i]);
    }
}

// Three functions whose addresses are taken to generate exactly three PLT stubs
void dummy_callback1(void) {
    uart_puts("Callback 1\n");
}

void dummy_callback2(void) {
    uart_puts("Callback 2\n");
}

void dummy_callback3(void) {
    uart_puts("Callback 3\n");
}

void _start(void) {
    // Taking the addresses of these 3 callbacks forces tool to generate
    // three PLT stubs in the final binary (__sseocb_g_plt_dummy_callback1..3).
    void (*volatile cb1)(void) = dummy_callback1;
    void (*volatile cb2)(void) = dummy_callback2;
    void (*volatile cb3)(void) = dummy_callback3;
    (void)cb1;
    (void)cb2;
    (void)cb3;

    uart_puts("--- First 30 Primes (3x10 Table) ---\n");

    int count = 0;
    int col = 0;
    unsigned int num = 2;

    while (count < 30) {
        if (is_prime(num)) {
            print_uint_padded(num, 5);
            count++;
            col++;
            if (col == 10) {
                uart_putc('\n');
                col = 0;
            }
        }
        num++;
    }

    __asm__ volatile("ebreak");
    while (1) {
    }
}

int main(void) {
    _start();
    return 0;
}
