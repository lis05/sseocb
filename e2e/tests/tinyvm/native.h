#ifndef TINYVM_NATIVE_H
#define TINYVM_NATIVE_H

void uart_putc(char c);
void uart_puts(const char *s);
void native_print_uint(unsigned int n);

#endif
