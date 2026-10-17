int virtual_handler(int n);

typedef int (*handler)(int);

int native_handler(int n) {
    volatile handler virtual = &virtual_handler;
    return virtual(n + 1);
}
