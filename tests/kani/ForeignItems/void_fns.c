// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// C functions returning `void`, for void_return.rs. They are plain C: Kani
// must link them as they are (no `struct Unit` return type).

#include <stdint.h>

uint32_t COUNTER = 0;

void increment(void) { COUNTER++; }

void set_counter(uint32_t value) { COUNTER = value; }

void add_into(uint32_t *out, uint32_t a, uint32_t b) { *out = a + b; }

void swap(uint32_t *a, uint32_t *b)
{
    uint32_t tmp = *a;
    *a           = *b;
    *b           = tmp;
}
