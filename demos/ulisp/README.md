# uLisp Bare-Metal Demo on SSEOCB

This demo runs **uLisp Zero 1.1** bare-metal on RV32E virtualized under **SSEOCB**.

## Files
* [`all.sh`](all.sh): Compilation and test runner script.
* [`program.lisp`](program.lisp): Lisp source code executed on boot.
* [`main.c`](main.c): Bare-metal C wrapper with software runtime builtins and setjmp/longjmp.
* [`memory.ld`](memory.ld): Bare-metal memory map (Flash, RAM, Stack, UART).
* [`ulisp/`](ulisp/): uLisp Zero core interpreter and MIT License.

## Usage

### Run Default Demo (Graemi decompressor)
```bash
./all.sh
```

### Run with Blaris (16 KiB Flash Target)
```bash
./all.sh --blaris
```

### Compare Native vs SSEOCB (Benchmark)
```bash
./all.sh --bench
```

### Run Native Directly
```bash
./all.sh --native
```

### Interactive Simulator Debugger
```bash
./all.sh --debug
```

### Changing the Lisp Program
Edit [`program.lisp`](program.lisp) with any valid uLisp expressions, then rerun `./all.sh`.

---

## Upstream Provenance & Diff with `uLispZero.ino`

The core interpreter in [`ulisp/ulisp_core.c`](ulisp/ulisp_core.c) is derived from upstream **[technoblogy/ulisp-zero](https://github.com/technoblogy/ulisp-zero)** (`uLispZero.ino`, MIT License by David Johnson-Davies).

Over **90%** of the code is verbatim original uLisp Zero, preserving the original:
* Cons-cell workspace & mark-and-sweep garbage collector
* Radix-40 symbol interning (packing up to 3 alphanumeric characters into 16 bits)
* Recursive descent S-expression parser
* Tree-walking evaluator, closures, environments, special forms, and built-ins

### Why changes were made:
1. **AVR Harvard Memory $\rightarrow$ Flat RISC-V Memory**:
   Original uLisp was written for 8-bit AVR (Arduino Uno). On AVR, Flash and RAM have separate address spaces, requiring AVR-specific GCC macros (`PGM_P`, `pgm_read_word()`, `strcpy_P()`) from `<avr/pgmspace.h>`. On 32-bit RISC-V, memory is unified, so these were replaced with standard C pointers (`const char*`, `strcpy`, `strcasecmp`).
2. **Arduino Serial $\rightarrow$ Standard C I/O**:
   Replaced Arduino `Serial.write()` / `Serial.read()` with standard C `putchar()` / `getchar()` mapped to memory-mapped UART.
3. **String Literals & Print Built-ins**:
   Original uLisp Zero only supported numbers and symbols. We added a `STRING` object type, `"..."` tokenization, and `princ` / `print` / `terpri` built-ins so Lisp programs can output text and ASCII art (like the 5-pointed star).
4. **EOF Handling**:
   Added `TOK_EOF` token so `read()` gracefully finishes when the embedded program ends.

### Key Diff

#### 1. String Types & Table Definitions
```diff
--- uLispZero.ino
+++ ulisp_core.c
@@ -29,11 +26,11 @@
 // Constants
-enum type { ZERO=0, SYMBOL=2, PAIR=4 };  // PAIR must be last
-enum token { UNUSED, BRA, KET, QUO, DOT };
+enum type { ZERO=0, SYMBOL=2, STRING=4, PAIR=6 };  // PAIR must be last
+enum token { UNUSED, BRA, KET, QUO, DOT, TOK_EOF };
 
 enum function { SYMBOLS, NIL, TEE, LAMBDA, SPECIAL_FORMS, QUOTE, DEFUN, DEFVAR, SETQ, IF, FUNCTIONS, NOT,
-NULLFN, CONS, ATOM, LISTP, CONSP, SYMBOLP, EQ, CAR, CDR, EVAL, GLOBALS, LOCALS, ENDFUNCTIONS };
+NULLFN, CONS, ATOM, LISTP, CONSP, SYMBOLP, EQ, CAR, CDR, EVAL, GLOBALS, LOCALS, PRINC, PRINT, TERPRI, ENDFUNCTIONS };
```

#### 2. Added `princ`, `print`, `terpri`
```diff
+object *fn_princ(object *args, object *env) {
+  (void) env;
+  object *obj = first(args);
+  printobject(obj);
+  return obj;
+}
+
+object *fn_print(object *args, object *env) {
+  (void) env;
+  object *obj = first(args);
+  pfl();
+  printobject(obj);
+  pln();
+  return obj;
+}
+
+object *fn_terpri(object *args, object *env) {
+  (void) args; (void) env;
+  pln();
+  return nil;
+}
```

#### 3. Replacing AVR `pgm_read_*` with Direct Pointers
```diff
-int builtin (char* n) {
+int builtin(char* n) {
   int entry = 0;
   while (entry < ENDFUNCTIONS) {
-   if (strcmp_P(n, (PGM_P)pgm_read_word(&lookup_table[entry].string)) == 0 )
+    if (strcasecmp(n, lookup_table[entry].string) == 0)
       return entry;
     entry++;
   }
   return ENDFUNCTIONS;
 }
 
-fn_ptr_type lookupfn (symbol_t name) {
-  return (fn_ptr_type)pgm_read_word(&lookup_table[name].fptr);
+fn_ptr_type lookupfn(symbol_t name) {
   return lookup_table[name].fptr;
 }
```

#### 4. I/O: Replacing Arduino `Serial` with `putchar()` / `getchar()`
```diff
-void pchar (char c) {
+void pchar(char c) {
   LastPrint = c;
-  Serial.write(c);
-  if (c == '\r') Serial.write('\n');
+  putchar(c);
 }
...
-int gchar () {
+int gchar(void) {
   if (LastChar) { 
-    char temp = LastChar;
+    int temp = LastChar;
     LastChar = 0;
     return temp;
   }
-  while (!Serial.available()) ;
-  char temp = Serial.read();
-  if (temp != '\r') pchar(temp);
+  int temp = getchar();
+  if (temp == -1) return -1;
+  if (temp != '\r') pchar((char)temp);
   return temp;
 }
```

#### 5. Parsing String Literals `"` in `nextitem()`
```diff
   if (ch == ')') return (object *)KET;
   if (ch == '(') return (object *)BRA;
   if (ch == '\'') return (object *)QUO;
   if (ch == '.') return (object *)DOT;
+  if (ch == '"') {
+    static char str_buf[1024];
+    static int str_pos = 0;
+    char *s = &str_buf[str_pos];
+    int c = gchar();
+    while (c != '"' && c != -1) {
+      if (str_pos < (int)sizeof(str_buf) - 1) {
+        str_buf[str_pos++] = (char)c;
+      }
+      c = gchar();
+    }
+    str_buf[str_pos++] = '\0';
+    return string(s);
+  }
```
