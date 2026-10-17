int ping(int n);

int pong(int n) {
    if (n <= 0) return 0;
    return ping(n - 1) + n;
}

