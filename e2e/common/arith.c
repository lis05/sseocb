#include "arith.h"

unsigned int udivmod(unsigned int n, unsigned int d, unsigned int *rem) {
    if (d == 0) {
        if (rem) {
            *rem = 0;
        }
        return 0;
    }
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

unsigned int __udivsi3(unsigned int n, unsigned int d) {
    return udivmod(n, d, 0);
}

int __divsi3(int a, int b) {
    int neg = 0;
    if (a < 0) {
        a = -a;
        neg = !neg;
    }
    if (b < 0) {
        b = -b;
        neg = !neg;
    }
    int res = (int)udivmod((unsigned int)a, (unsigned int)b, 0);
    return neg ? -res : res;
}

int __modsi3(int a, int b) {
    int neg = (a < 0);
    if (a < 0) {
        a = -a;
    }
    if (b < 0) {
        b = -b;
    }
    unsigned int rem = 0;
    udivmod((unsigned int)a, (unsigned int)b, &rem);
    return neg ? -(int)rem : (int)rem;
}

unsigned int __umodsi3(unsigned int a, unsigned int b) {
    unsigned int rem = 0;
    udivmod(a, b, &rem);
    return rem;
}
