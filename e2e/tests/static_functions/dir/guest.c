typedef int (*handler)(int);

static int a(int n) {
    return n + 15;
}

handler get_dir_a(void) {
    return a;
}
