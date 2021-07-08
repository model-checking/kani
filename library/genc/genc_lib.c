// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#include <assert.h>
#include <string.h>
#include <limits.h>
#include <stdio.h>

void __CPROVER_assume(int condition) {
    assert(condition);
}

#define OBJECT_SIZE(value) sizeof(*value)



// #define overflow(op, typ, var1, var2) \
//     (strcmp(op, "+") == 0) ? ( \
//         ((var1 < 0) && (var2 < 0) && (var1 + var2 > 0)) || \
//         ((var1 > 0) && (var2 > 0) && (var1 + var2 < 0)) \
//     ) : (strcmp(op, "-") == 0) ? ( \
//         ((var1 < 0) && (var2 > 0) && (var1 - var2 > 0)) || \
//         ((var1 > 0) && (var2 < 0) && (var1 - var2 < 0)) \
//     ) : (strcmp(op, "*") == 0) ? ( \
//         (var1 != 0) && ((var1 * var2) / var1 != var2) \
//     ) : ( \
//         1 \
//     )

// nondet_<k>
#define byte_extract_little_endian(from_val, offset, to_type) \
    ((to_type) from_val)

typedef struct X {
    int y;
} X;

int main() {
    X x = { 5 };
    int y = byte_extract_little_endian(x, 0, int);
}