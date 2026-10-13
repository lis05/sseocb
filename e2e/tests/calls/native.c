extern int virtual(int sum, int cnt);

int native(int sum, int cnt) {
    if (cnt < 0) {
        return sum;
    }
    return virtual(sum + cnt * cnt, cnt - 2);
}

int __mulsi3(int a, int b) {
    int res = 0;
    int neg = 0;
    if (b < 0) {
        b = -b;
        neg = !neg;
    }
    while (b > 0) {
        if (b & 1) {
            res += a;
        }
        a <<= 1;
        b >>= 1;
    }
    return neg ? -res : res;
}
