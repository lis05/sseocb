#include "misc.h"

void exit(int code) {
    register int a0_reg __asm__("a0") = code;
    __asm__ volatile("ebreak" : : "r"(a0_reg));
    while (1) {
    }
}
