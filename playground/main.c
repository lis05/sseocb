// ============================================================================
// Simple SSEOCB Guest Test Program
// ============================================================================

extern int printf(const char *fmt, ...);

// 1. Address-taken function (placed inside .__sseocb_g_vP at 0x800000XX)
int my_callback(int a) {
    return a + 1;
}

// 2. Static data member (placed in physical .data, NOT inside .__sseocb_g_vP)
static int s_counter = 42;

// Function pointer taking the address of my_callback in .data table
typedef int (*calc_fn_t)(int);
calc_fn_t g_fn_ptr = my_callback;

void _start(void) {
    // Taking address of my_callback in code (emits loads targeting 0x800000XX)
    calc_fn_t fn = my_callback;

    // Using the static data member
    s_counter += fn(10);

    // Call external native function
    printf("Counter: %d\n", s_counter);

    while (1) {
        // Idle
    }
}
