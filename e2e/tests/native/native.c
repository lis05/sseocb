#define SIEVE_BYTES 6600

static unsigned char sieve[SIEVE_BYTES];
static int sieved = 0;

static void init_sieve(void) {
    if (sieved) {
        return;
    }
    sieved = 1;

    for (int i = 0; i < SIEVE_BYTES; i++) {
        sieve[i] = 0;
    }

    for (unsigned int p = 3; p * p <= 104729; p += 2) {
        unsigned int idx = (p - 3) >> 1;
        if ((sieve[idx >> 3] & (1 << (idx & 7))) == 0) {
            for (unsigned int m = p * p; m <= 104729; m += (p << 1)) {
                unsigned int m_idx = (m - 3) >> 1;
                sieve[m_idx >> 3] |= (1 << (m_idx & 7));
            }
        }
    }
}

unsigned int __mulsi3(unsigned int a, unsigned int b) {
    unsigned int res = 0;
    while (b > 0) {
        if (b & 1) {
            res += a;
        }
        a <<= 1;
        b >>= 1;
    }
    return res;
}

int is_prime(unsigned int n) {
    if (!sieved) {
        init_sieve();
    }
    if (n < 2) {
        return 0;
    }
    if (n == 2) {
        return 1;
    }
    if ((n & 1) == 0) {
        return 0;
    }
    if (n > 104729) {
        return 0;
    }
    unsigned int idx = (n - 3) >> 1;
    return (sieve[idx >> 3] & (1 << (idx & 7))) == 0;
}
