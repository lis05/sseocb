#include <arith.h>

int is_prime(unsigned int n) {
    if (n < 2) {
        return 0;
    }
    for (unsigned int d = 2; ; d++) {
        unsigned int rem = 0;
        unsigned int q = udivmod(n, d, &rem);
        if (d > q) {
            break;
        }
        if (rem == 0) {
            return 0;
        }
    }
    return 1;
}
