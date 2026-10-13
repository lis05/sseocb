#ifndef E2E_ARITH_H
#define E2E_ARITH_H

static inline unsigned int udivmod(unsigned int n, unsigned int d, unsigned int *rem) {
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

#endif
