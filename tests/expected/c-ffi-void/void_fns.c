// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#include <stdint.h>

void decrement(uint32_t *p) { *p -= 1; }

void add_one(uint32_t *p) { *p += 1; }

void double_it(uint32_t *p) { *p *= 2; }

// Does not return (like `exit`).
void halt(void) { __CPROVER_assume(0); }

// Returns, although main.rs declares it as not returning.
void returns_anyway(void) {}
