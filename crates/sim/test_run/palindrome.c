#include "uart.h"

static int is_palindrome(const char *s) {
    int len = 0;
    while (s[len]) len++;
    if (len == 0) return 1;
    int i = 0, j = len - 1;
    while (i < j) {
        if (s[i] != s[j]) return 0;
        i++;
        j--;
    }
    return 1;
}

static void check_str(const char *s) {
    uart_puts("\"");
    uart_puts(s);
    uart_puts("\" -> ");
    if (is_palindrome(s)) {
        uart_puts("YES\n");
    } else {
        uart_puts("NO\n");
    }
}

int main(void) {
    uart_puts("Palindrome test:\n");
    check_str("racecar");
    check_str("hello");
    check_str("madam");
    check_str("riscv");
    check_str("step on no pets");
    check_str("noon");
    check_str("code");
    check_str("a");
    check_str("");
    uart_puts("Palindrome done.\n");
    return 0;
}
