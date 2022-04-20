// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

// This is a C implementation of the __rust_realloc function that has the following signature
//     fn __rust_realloc(ptr: *mut u8, old_size: usize, align: usize, new_size: usize) -> *mut u8;
// The implementation pretty much follows CBMC's implementation:
// https://github.com/diffblue/cbmc/blob/94b81b22799876b52632ba9cdc3f486f87abadb8/src/ansi-c/library/stdlib.c#L472
uint8_t* __rust_realloc(uint8_t* ptr, size_t old_size, size_t align, size_t new_size) {
    // if current ptr is NULL, this behaves like malloc
    if (ptr == 0)
        return malloc(new_size);

    // if malloc-size is 0, free original object and return malloc(0) which
    // returns an invalid pointer
    if (new_size == 0) {
        free(ptr);
        return malloc(0);
    }

    uint8_t* result = malloc(new_size);
    if (result) {
        size_t bytes_to_copy = new_size < old_size ? new_size : old_size;
        memcpy(result, ptr, bytes_to_copy);
        free(ptr);
    }

    return result;
}
