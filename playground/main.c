#define UART_BASE 0x40000000

static inline void uart_putc(char c) {
    *(volatile char *)UART_BASE = c;
}

static inline void uart_puts(const char *s) {
    while (*s) {
        uart_putc(*s++);
    }
}

static void uart_put_hex8(unsigned char b) {
    const char *hex = "0123456789abcdef";
    uart_putc(hex[(b >> 4) & 0xf]);
    uart_putc(hex[b & 0xf]);
}

void *memcpy(void *dest, const void *src, unsigned int n) {
    char *d = (char *)dest;
    const char *s = (const char *)src;
    while (n--) *d++ = *s++;
    return dest;
}

void *memset(void *dest, int c, unsigned int n) {
    char *d = (char *)dest;
    while (n--) *d++ = (char)c;
    return dest;
}

static inline unsigned int my_strlen(const char *s) {
    unsigned int len = 0;
    while (*s++) len++;
    return len;
}

static int my_memcmp(const void *s1, const void *s2, unsigned int n) {
    const unsigned char *p1 = (const unsigned char *)s1;
    const unsigned char *p2 = (const unsigned char *)s2;
    for (unsigned int i = 0; i < n; i++) {
        if (p1[i] != p2[i]) return (int)p1[i] - (int)p2[i];
    }
    return 0;
}

/* Freestanding SHA-256 (32-bit ARX only: no mul, no div, no 64-bit builtins) */
#define ROTR(x, n) (((x) >> (n)) | ((x) << (32 - (n))))
#define CH(x, y, z) (((x) & (y)) ^ (~(x) & (z)))
#define MAJ(x, y, z) (((x) & (y)) ^ ((x) & (z)) ^ ((y) & (z)))
#define SIG0(x) (ROTR(x, 2) ^ ROTR(x, 13) ^ ROTR(x, 22))
#define SIG1(x) (ROTR(x, 6) ^ ROTR(x, 11) ^ ROTR(x, 25))
#define sig0(x) (ROTR(x, 7) ^ ROTR(x, 18) ^ ((x) >> 3))
#define sig1(x) (ROTR(x, 17) ^ ROTR(x, 19) ^ ((x) >> 10))

static const unsigned int K[64] = {
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
};

typedef struct {
    unsigned int state[8];
    unsigned int count;
    unsigned char buffer[64];
} SHA256_CTX;

__attribute__((noinline))
void sha256_transform(unsigned int state[8], const unsigned char data[64]) {
    unsigned int a = state[0], b = state[1], c = state[2], d = state[3];
    unsigned int e = state[4], f = state[5], g = state[6], h = state[7];
    unsigned int w[64];

    for (int i = 0; i < 16; i++) {
        w[i] = ((unsigned int)data[i * 4] << 24)
             | ((unsigned int)data[i * 4 + 1] << 16)
             | ((unsigned int)data[i * 4 + 2] << 8)
             | ((unsigned int)data[i * 4 + 3]);
    }
    for (int i = 16; i < 64; i++) {
        w[i] = sig1(w[i - 2]) + w[i - 7] + sig0(w[i - 15]) + w[i - 16];
    }

    for (int i = 0; i < 64; i++) {
        unsigned int t1 = h + SIG1(e) + CH(e, f, g) + K[i] + w[i];
        unsigned int t2 = SIG0(a) + MAJ(a, b, c);
        h = g;
        g = f;
        f = e;
        e = d + t1;
        d = c;
        c = b;
        b = a;
        a = t1 + t2;
    }

    state[0] += a;
    state[1] += b;
    state[2] += c;
    state[3] += d;
    state[4] += e;
    state[5] += f;
    state[6] += g;
    state[7] += h;
}

void sha256_init(SHA256_CTX *ctx) {
    ctx->state[0] = 0x6a09e667;
    ctx->state[1] = 0xbb67ae85;
    ctx->state[2] = 0x3c6ef372;
    ctx->state[3] = 0xa54ff53a;
    ctx->state[4] = 0x510e527f;
    ctx->state[5] = 0x9b05688c;
    ctx->state[6] = 0x1f83d9ab;
    ctx->state[7] = 0x5be0cd19;
    ctx->count = 0;
}

void sha256_update(SHA256_CTX *ctx, const unsigned char *data, unsigned int len) {
    unsigned int index = ctx->count & 0x3f;
    ctx->count += len;

    unsigned int part_len = 64 - index;
    unsigned int i = 0;

    if (len >= part_len) {
        memcpy(&ctx->buffer[index], data, part_len);
        sha256_transform(ctx->state, ctx->buffer);
        for (i = part_len; i + 63 < len; i += 64) {
            sha256_transform(ctx->state, &data[i]);
        }
        index = 0;
    }
    memcpy(&ctx->buffer[index], &data[i], len - i);
}

void sha256_final(SHA256_CTX *ctx, unsigned char digest[32]) {
    unsigned char final_count[8] = { 0 };
    unsigned int bit_count = ctx->count << 3;
    final_count[4] = (unsigned char)((bit_count >> 24) & 0xff);
    final_count[5] = (unsigned char)((bit_count >> 16) & 0xff);
    final_count[6] = (unsigned char)((bit_count >> 8) & 0xff);
    final_count[7] = (unsigned char)(bit_count & 0xff);

    unsigned int index = ctx->count & 0x3f;
    unsigned int pad_len = (index < 56) ? (56 - index) : (120 - index);

    static const unsigned char padding[64] = { 0x80 };
    sha256_update(ctx, padding, pad_len);
    sha256_update(ctx, final_count, 8);

    for (int i = 0; i < 8; i++) {
        digest[i * 4] = (unsigned char)((ctx->state[i] >> 24) & 0xff);
        digest[i * 4 + 1] = (unsigned char)((ctx->state[i] >> 16) & 0xff);
        digest[i * 4 + 2] = (unsigned char)((ctx->state[i] >> 8) & 0xff);
        digest[i * 4 + 3] = (unsigned char)(ctx->state[i] & 0xff);
    }
}

static const unsigned char EXPECTED_1[32] = {
    0xd7, 0xa8, 0xfb, 0xb3, 0x07, 0xd7, 0x80, 0x94,
    0x69, 0xca, 0x9a, 0xbc, 0xb0, 0x08, 0x2e, 0x4f,
    0x8d, 0x56, 0x51, 0xe4, 0x6d, 0x3c, 0xdb, 0x76,
    0x2d, 0x02, 0xd0, 0xbf, 0x37, 0xc9, 0xe5, 0x92
};

static const unsigned char EXPECTED_2[32] = {
    0x2e, 0x14, 0xe3, 0x47, 0x08, 0xbd, 0xcb, 0x53,
    0x5c, 0x80, 0x0d, 0xa9, 0xd4, 0x8e, 0xbf, 0x3e,
    0xd0, 0x87, 0xec, 0x98, 0x73, 0x75, 0xfb, 0xeb,
    0x0a, 0x18, 0xf2, 0xdd, 0x92, 0x5b, 0xce, 0xb5
};

void _start(void) {
    uart_puts("--- SSEOCB SHA-256 Benchmark ---\n");

    SHA256_CTX ctx;
    unsigned char digest[32];

    // Test 1: Single block NIST test vector
    uart_puts("Test 1 (NIST standard test vector):\n");
    const char *msg1 = "The quick brown fox jumps over the lazy dog";
    sha256_init(&ctx);
    sha256_update(&ctx, (const unsigned char *)msg1, my_strlen(msg1));
    sha256_final(&ctx, digest);

    uart_puts("  Hash: ");
    for (int i = 0; i < 32; i++) uart_put_hex8(digest[i]);
    if (my_memcmp(digest, EXPECTED_1, 32) == 0) {
        uart_puts(" [PASS]\n");
    } else {
        uart_puts(" [FAIL]\n");
    }

    // Test 2: Multi-block 2400-byte benchmark (38 blocks, 2432 mixing rounds)
    uart_puts("Test 2 (Multi-block 2.4 KiB, 2432 mixing rounds):\n");
    sha256_init(&ctx);
    const char *chunk = "SSEOCB-SHA256-BENCHMARK-";
    unsigned int chunk_len = my_strlen(chunk);
    for (int k = 0; k < 100; k++) {
        sha256_update(&ctx, (const unsigned char *)chunk, chunk_len);
    }
    sha256_final(&ctx, digest);

    uart_puts("  Hash: ");
    for (int i = 0; i < 32; i++) uart_put_hex8(digest[i]);
    if (my_memcmp(digest, EXPECTED_2, 32) == 0) {
        uart_puts(" [PASS]\n");
    } else {
        uart_puts(" [FAIL]\n");
    }

    uart_puts("--- Benchmark Complete ---\n");
    __asm__ volatile("ebreak");
    while (1) {
    }
}

int main(void) {
    _start();
    return 0;
}
