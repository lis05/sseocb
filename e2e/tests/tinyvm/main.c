#include "native.h"

// Software 32-bit integer multiplication (rv32e has no hardware multiplier)
int __mulsi3(int a, int b) {
    int res = 0;
    unsigned int ua = (unsigned int)a;
    unsigned int ub = (unsigned int)b;
    while (ub > 0) {
        if (ub & 1) {
            res += (int)ua;
        }
        ua <<= 1;
        ub >>= 1;
    }
    return res;
}

// Software 32-bit unsigned division and modulo (rv32e has no hardware divider)
static unsigned int udivmod(unsigned int n, unsigned int d, unsigned int *rem) {
    if (d == 0) return 0;
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

static int sdiv(int a, int b) {
    if (b == 0) return 0;
    int sign = 1;
    unsigned int ua = (unsigned int)a;
    unsigned int ub = (unsigned int)b;
    if (a < 0) { sign = -sign; ua = (unsigned int)(-a); }
    if (b < 0) { sign = -sign; ub = (unsigned int)(-b); }
    unsigned int q = udivmod(ua, ub, 0);
    return sign < 0 ? -(int)q : (int)q;
}

static int smod(int a, int b) {
    if (b == 0) return 0;
    int sign = a < 0 ? -1 : 1;
    unsigned int ua = a < 0 ? (unsigned int)(-a) : (unsigned int)a;
    unsigned int ub = b < 0 ? (unsigned int)(-b) : (unsigned int)b;
    unsigned int rem = 0;
    udivmod(ua, ub, &rem);
    return sign < 0 ? -(int)rem : (int)rem;
}

// Opcode definitions for the TinyVM stack-based bytecode language
enum Opcode {
    OP_HALT = 0,
    OP_PUSH,
    OP_POP,
    OP_DUP,
    OP_SWAP,
    OP_OVER,
    OP_ADD,
    OP_SUB,
    OP_MUL,
    OP_DIV,
    OP_MOD,
    OP_AND,
    OP_OR,
    OP_XOR,
    OP_EQ,
    OP_LT,
    OP_GT,
    OP_LE,
    OP_GE,
    OP_JMP,
    OP_JZ,
    OP_JNZ,
    OP_CALL,
    OP_RET,
    OP_LOAD,
    OP_STORE,
    OP_PRINT_NUM,
    OP_PRINT_CHAR,
    OP_PRINT_STR,
};

#define STACK_CAP 64
#define CALL_STACK_CAP 32
#define MEM_CAP 64

typedef struct {
    int stack[STACK_CAP];
    int sp;

    int call_stack[CALL_STACK_CAP];
    int csp;

    int memory[MEM_CAP];
    int pc;
    int halted;
} VM;

static void vm_init(VM *vm) {
    vm->sp = 0;
    vm->csp = 0;
    vm->pc = 0;
    vm->halted = 0;
    for (int i = 0; i < MEM_CAP; i++) {
        vm->memory[i] = 0;
    }
}

static void vm_push(VM *vm, int val) {
    if (vm->sp < STACK_CAP) {
        vm->stack[vm->sp++] = val;
    }
}

static int vm_pop(VM *vm) {
    if (vm->sp > 0) {
        return vm->stack[--vm->sp];
    }
    return 0;
}

static void vm_run(VM *vm, const int *code, int code_len, const char **strings) {
    while (!vm->halted && vm->pc < code_len) {
        int op = code[vm->pc++];
        switch (op) {
            case OP_HALT:
                vm->halted = 1;
                break;
            case OP_PUSH:
                vm_push(vm, code[vm->pc++]);
                break;
            case OP_POP:
                vm_pop(vm);
                break;
            case OP_DUP:
                if (vm->sp > 0) {
                    vm_push(vm, vm->stack[vm->sp - 1]);
                }
                break;
            case OP_SWAP:
                if (vm->sp >= 2) {
                    int top = vm->stack[vm->sp - 1];
                    vm->stack[vm->sp - 1] = vm->stack[vm->sp - 2];
                    vm->stack[vm->sp - 2] = top;
                }
                break;
            case OP_OVER:
                if (vm->sp >= 2) {
                    vm_push(vm, vm->stack[vm->sp - 2]);
                }
                break;
            case OP_ADD: {
                int b = vm_pop(vm);
                int a = vm_pop(vm);
                vm_push(vm, a + b);
                break;
            }
            case OP_SUB: {
                int b = vm_pop(vm);
                int a = vm_pop(vm);
                vm_push(vm, a - b);
                break;
            }
            case OP_MUL: {
                int b = vm_pop(vm);
                int a = vm_pop(vm);
                vm_push(vm, a * b);
                break;
            }
            case OP_DIV: {
                int b = vm_pop(vm);
                int a = vm_pop(vm);
                vm_push(vm, sdiv(a, b));
                break;
            }
            case OP_MOD: {
                int b = vm_pop(vm);
                int a = vm_pop(vm);
                vm_push(vm, smod(a, b));
                break;
            }
            case OP_AND: {
                int b = vm_pop(vm);
                int a = vm_pop(vm);
                vm_push(vm, a & b);
                break;
            }
            case OP_OR: {
                int b = vm_pop(vm);
                int a = vm_pop(vm);
                vm_push(vm, a | b);
                break;
            }
            case OP_XOR: {
                int b = vm_pop(vm);
                int a = vm_pop(vm);
                vm_push(vm, a ^ b);
                break;
            }
            case OP_EQ: {
                int b = vm_pop(vm);
                int a = vm_pop(vm);
                vm_push(vm, a == b ? 1 : 0);
                break;
            }
            case OP_LT: {
                int b = vm_pop(vm);
                int a = vm_pop(vm);
                vm_push(vm, a < b ? 1 : 0);
                break;
            }
            case OP_GT: {
                int b = vm_pop(vm);
                int a = vm_pop(vm);
                vm_push(vm, a > b ? 1 : 0);
                break;
            }
            case OP_LE: {
                int b = vm_pop(vm);
                int a = vm_pop(vm);
                vm_push(vm, a <= b ? 1 : 0);
                break;
            }
            case OP_GE: {
                int b = vm_pop(vm);
                int a = vm_pop(vm);
                vm_push(vm, a >= b ? 1 : 0);
                break;
            }
            case OP_JMP:
                vm->pc = code[vm->pc];
                break;
            case OP_JZ: {
                int target = code[vm->pc++];
                if (vm_pop(vm) == 0) {
                    vm->pc = target;
                }
                break;
            }
            case OP_JNZ: {
                int target = code[vm->pc++];
                if (vm_pop(vm) != 0) {
                    vm->pc = target;
                }
                break;
            }
            case OP_CALL: {
                int target = code[vm->pc++];
                if (vm->csp < CALL_STACK_CAP) {
                    vm->call_stack[vm->csp++] = vm->pc;
                    vm->pc = target;
                }
                break;
            }
            case OP_RET:
                if (vm->csp > 0) {
                    vm->pc = vm->call_stack[--vm->csp];
                }
                break;
            case OP_LOAD: {
                int addr = code[vm->pc++];
                if (addr >= 0 && addr < MEM_CAP) {
                    vm_push(vm, vm->memory[addr]);
                } else {
                    vm_push(vm, 0);
                }
                break;
            }
            case OP_STORE: {
                int addr = code[vm->pc++];
                int val = vm_pop(vm);
                if (addr >= 0 && addr < MEM_CAP) {
                    vm->memory[addr] = val;
                }
                break;
            }
            case OP_PRINT_NUM: {
                int val = vm_pop(vm);
                if (val < 0) {
                    uart_putc('-');
                    val = -val;
                }
                native_print_uint((unsigned int)val);
                break;
            }
            case OP_PRINT_CHAR: {
                char c = (char)code[vm->pc++];
                uart_putc(c);
                break;
            }
            case OP_PRINT_STR: {
                int idx = code[vm->pc++];
                uart_puts(strings[idx]);
                break;
            }
            default:
                vm->halted = 1;
                break;
        }
    }
}

static const char *STRINGS[] = {
    "--- TinyVM Starting ---\n",
    "Arithmetic (15 + 25) * 3 = ",
    "Bitwise (255 ^ 15) & 170 = ",
    "Memory Array Sum = ",
    "Factorial(4) = ",
    "Fibonacci(5): ",
    "--- TinyVM Finished ---\n",
};

static const int BYTECODE[] = {
    // 0: Entry point -> jump to main
    OP_JMP, 20,

    // 2: Subroutine fact(n) -> n!
    // stack: [..., n] -> [..., n!]
    OP_DUP, OP_PUSH, 1, OP_LE, OP_JZ, 12,
    OP_POP, OP_PUSH, 1, OP_RET,
    // 12: fact_rec
    OP_DUP, OP_PUSH, 1, OP_SUB,
    OP_CALL, 2, OP_MUL, OP_RET,

    // 20: main
    // 1. Banner
    OP_PRINT_STR, 0,

    // 2. Arithmetic: (15 + 25) * 3 = 120
    OP_PRINT_STR, 1,
    OP_PUSH, 15, OP_PUSH, 25, OP_ADD,
    OP_PUSH, 3, OP_MUL,
    OP_PRINT_NUM, OP_PRINT_CHAR, '\n',

    // 3. Bitwise: (255 ^ 15) & 170 = 160
    OP_PRINT_STR, 2,
    OP_PUSH, 255, OP_PUSH, 15, OP_XOR,
    OP_PUSH, 170, OP_AND,
    OP_PRINT_NUM, OP_PRINT_CHAR, '\n',

    // 4. Memory array sum: mem[0]=10, mem[1]=20, mem[2]=30; sum = 60
    OP_PRINT_STR, 3,
    OP_PUSH, 10, OP_STORE, 0,
    OP_PUSH, 20, OP_STORE, 1,
    OP_PUSH, 30, OP_STORE, 2,
    OP_LOAD, 0, OP_LOAD, 1, OP_ADD,
    OP_LOAD, 2, OP_ADD,
    OP_PRINT_NUM, OP_PRINT_CHAR, '\n',

    // 5. Factorial(4) = 24
    OP_PRINT_STR, 4,
    OP_PUSH, 4, OP_CALL, 2,
    OP_PRINT_NUM, OP_PRINT_CHAR, '\n',

    // 6. Fibonacci: 5 numbers: 0 1 1 2 3
    OP_PRINT_STR, 5,
    OP_PUSH, 0, OP_STORE, 0,
    OP_PUSH, 1, OP_STORE, 1,
    OP_PUSH, 5, OP_STORE, 2,
    // 96: fib_loop
    OP_LOAD, 2, OP_JZ, 129,
    OP_LOAD, 0, OP_PRINT_NUM, OP_PRINT_CHAR, ' ',
    OP_LOAD, 0, OP_LOAD, 1, OP_ADD, OP_STORE, 3,
    OP_LOAD, 1, OP_STORE, 0,
    OP_LOAD, 3, OP_STORE, 1,
    OP_LOAD, 2, OP_PUSH, 1, OP_SUB, OP_STORE, 2,
    OP_JMP, 96,
    // 129: fib_done
    OP_PRINT_CHAR, '\n',

    // Done
    OP_PRINT_STR, 6,
    OP_HALT,
};

void _start(void) {
    VM vm;
    vm_init(&vm);

    int code_len = sizeof(BYTECODE) / sizeof(BYTECODE[0]);
    vm_run(&vm, BYTECODE, code_len, STRINGS);

    register int a0_reg __asm__("a0") = 0;
    __asm__ volatile("ebreak" : : "r"(a0_reg));
    while (1) {
    }
}
