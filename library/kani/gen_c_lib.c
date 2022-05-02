// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This file contains stubs that we link to the produced C file
//! from --gen-c-runnable to make it executable.

#include <assert.h>
#include <limits.h>
#include <math.h>
#include <stdbool.h>
#include <stdio.h>

// By default, don't do anything;
// user can add an assert if they so desire.
void __CPROVER_assume(int condition) {}

// We can ignore atomics for simplicity
void __CPROVER_atomic_begin(void) {}

void __CPROVER_atomic_end(void) {}

// These need to be manually defined for some reason
double powi(double base, int expt) { return pow(base, ( double )expt); }

float powif(float base, int expt) { return ( float )powi(( double )base, expt); }

// Used by cbmc invariants
typedef bool __CPROVER_bool;

#define OBJECT_SIZE(value) sizeof(*value)

// POINTER_OBJECT is used by Rust's offset_from to ensure
// the two pointers are from the same object.
// We can't do this in C.
// Tracking issue: https://github.com/model-checking/kani/issues/440
#define POINTER_OBJECT(value) 0

// Use built-in overflow operators
#define BUILTIN_ADD_OVERFLOW(var1, var2)           \
    ({                                             \
        int _tmp = 0;                              \
        __builtin_add_overflow(var1, var2, &_tmp); \
    })
#define BUILTIN_SUB_OVERFLOW(var1, var2)           \
    ({                                             \
        int _tmp = 0;                              \
        __builtin_sub_overflow(var1, var2, &_tmp); \
    })
#define BUILTIN_MUL_OVERFLOW(var1, var2)           \
    ({                                             \
        int _tmp = 0;                              \
        __builtin_mul_overflow(var1, var2, &_tmp); \
    })

#define overflow(op, typ, var1, var2)                      \
    ((op[ 0 ] == '+')   ? BUILTIN_ADD_OVERFLOW(var1, var2) \
     : (op[ 0 ] == '-') ? BUILTIN_SUB_OVERFLOW(var1, var2) \
     : (op[ 0 ] == '*') ? BUILTIN_MUL_OVERFLOW(var1, var2) \
                        : 1)

// Only works on little endian machines.
#define byte_extract_little_endian(from_val, offset, to_type) *(( typeof(to_type) * )((( void * )&from_val) + (offset)))
