#include <assert.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

size_t my_add(size_t num, ...)
{
    va_list argp;
    va_start(argp, num);

    size_t accum = 0;
    for (size_t i = 0; i < num; ++i) {
        size_t next = va_arg(argp, size_t);
        accum += next;
    }
    va_end(argp);
    return accum;
}

int my_add2(size_t num, ...)
{
    va_list argp;
    va_start(argp, num);

    int accum = 0;
    for (int i = 0; i < num; ++i) {
        int next = va_arg(argp, int);
        accum += next;
    }
    va_end(argp);
    return accum;
}

struct Foo {
    unsigned int  i;
    unsigned char c;
};  // __attribute__((packed));

struct Foo2 {
    uint32_t i;
    uint8_t  c;
    uint32_t i2;
};  // __attribute__((packed));

uint32_t S = 12;

void update_static() { S++; }

uint32_t takes_int(uint32_t i) { return i + 2; }

uint32_t takes_ptr(uint32_t *p) { return *p + 2; }

uint32_t takes_ptr_option(uint32_t *p)
{
    if (p) {
        return *p - 1;
    } else {
        return 0;
    }
}

void mutates_ptr(uint32_t *p) { *p -= 1; }

uint32_t name_in_c(uint32_t i) { return i + 2; }

uint32_t takes_struct(struct Foo f) { return f.i + f.c; }

uint32_t takes_struct_ptr(struct Foo *f) { return f->i + f->c; }

uint32_t takes_struct2(struct Foo2 f)
{
    assert(sizeof(unsigned int) == sizeof(uint32_t));
    return f.i + f.i2;
}

uint32_t takes_struct_ptr2(struct Foo2 *f) { return f->i + f->c; }
