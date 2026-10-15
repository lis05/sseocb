#ifndef _STDLIB_H_
#define _STDLIB_H_

#define NULL ((void*)0)

static inline int atoi(const char *str) {
    int res = 0;
    int sign = 1;
    while (*str == ' ' || *str == '\t' || *str == '\n') str++;
    if (*str == '-') { sign = -1; str++; }
    else if (*str == '+') { str++; }
    while (*str >= '0' && *str <= '9') {
        res = res * 10 + (*str - '0');
        str++;
    }
    return res * sign;
}

void *malloc(unsigned int size);
void free(void *ptr);
void exit(int status);

#endif
