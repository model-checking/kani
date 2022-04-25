// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

// This is a C implementation of the __rust_realloc function that has the following signature:
//     fn __rust_realloc(ptr: *mut u8, old_size: usize, align: usize, new_size: usize) -> *mut u8;
// This low-level function is called by std::alloc:realloc, and its
// implementation is provided by the compiler backend, so we need to provide an
// implementation for it to prevent verification failure due to missing function
// definition.
// For safety, refer to the documentation of GlobalAlloc::realloc:
// https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html#method.realloc
uint8_t *__rust_realloc(uint8_t *ptr, size_t old_size, size_t align, size_t new_size)
{
    // Passing a NULL pointer is undefined behavior
    __CPROVER_assert(ptr != 0, "realloc called with a null pointer");
    __CPROVER_assume(ptr != 0);

    // Passing a new_size of 0 is undefined behavior
    __CPROVER_assert(new_size > 0, "realloc called with a size of 0");
    __CPROVER_assume(new_size > 0);

    uint8_t *result = malloc(new_size);
    if (result) {
        size_t bytes_to_copy = new_size < old_size ? new_size : old_size;
        memcpy(result, ptr, bytes_to_copy);
        free(ptr);
    }

    return result;
}
