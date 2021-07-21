// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#include <assert.h>
#include <limits.h>
#include <math.h>
#include <stdio.h>
#include <stdbool.h>
#include <string.h>

void __CPROVER_assume(int condition) {
    assert(condition);
}

void __CPROVER_atomic_begin(void) {
}

void __CPROVER_atomic_end(void) {
}

double powi(double base, int expt) {
    return pow(base, (double) expt);
}

float powif(float base, int expt) {
    return (float) powi((double) base, expt);
}

typedef bool __CPROVER_bool;

#define OBJECT_SIZE(value) sizeof(*value)

// A temporary definition to always cause checks to be true.
#define POINTER_OBJECT(value) 0

#define overflow(op, typ, var1, var2) \
    (strcmp(op, "+") == 0) ? ( \
        ((var1 < 0) && (var2 < 0) && (var1 + var2 > 0)) || \
        ((var1 > 0) && (var2 > 0) && (var1 + var2 < 0)) \
    ) : (strcmp(op, "-") == 0) ? ( \
        ((var1 < 0) && (var2 > 0) && (var1 - var2 > 0)) || \
        ((var1 > 0) && (var2 < 0) && (var1 - var2 < 0)) \
    ) : (strcmp(op, "*") == 0) ? ( \
        (var1 != 0) && ((var1 * var2) / var1 != var2) \
    ) : ( \
        1 \
    )

#define byte_extract_little_endian(from_val, offset, to_type) \
    *((typeof(to_type)*) (((void*) &from_val) + (offset)))

