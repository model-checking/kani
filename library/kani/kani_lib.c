// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

// This is a C implementation of the __rust_alloc function.
// https://stdrs.dev/nightly/x86_64-unknown-linux-gnu/alloc/alloc/fn.__rust_alloc.html
// It has the following Rust signature:
//   `unsafe fn __rust_alloc(size: usize, align: usize) -> *mut u8`
// This low-level function is called by std::alloc:alloc, and its
// implementation is provided by the compiler backend, so we need to provide an
// implementation for it to prevent verification failure due to missing function
// definition.
// For safety, refer to the documentation of GlobalAlloc::alloc:
// https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html#tymethod.alloc
uint8_t *__rust_alloc(size_t size, size_t align)
{
    __CPROVER_assert(size > 0, "__rust_alloc must be called with a size greater than 0");
    __CPROVER_assume(size > 0);
    // Note: we appear to do nothing with `align`
    // TODO: https://github.com/model-checking/kani/issues/1168
    return malloc(size);
}

// This is a C implementation of the __rust_alloc_zeroed function.
// https://stdrs.dev/nightly/x86_64-unknown-linux-gnu/alloc/alloc/fn.__rust_alloc_zeroed.html
// It has the following Rust signature:
//   unsafe fn __rust_alloc_zeroed(size: usize, align: usize) -> *mut u8
// This low-level function is called by std::alloc:alloc_zeroed, and its
// implementation is provided by the compiler backend, so we need to provide an
// implementation for it to prevent verification failure due to missing function
// definition.
// For safety, refer to the documentation of GlobalAlloc::alloc_zeroed:
// hhttps://doc.rust-lang.org/std/alloc/fn.alloc_zeroed.html
uint8_t *__rust_alloc_zeroed(size_t size, size_t align)
{
    __CPROVER_assert(size > 0, "__rust_alloc_zeroed must be called with a size greater than 0");
    __CPROVER_assume(size > 0);
    // Note: we appear to do nothing with `align`
    // TODO: https://github.com/model-checking/kani/issues/1168
    return calloc(1, size);
}

// This is a C implementation of the __rust_dealloc function.
// https://stdrs.dev/nightly/x86_64-unknown-linux-gnu/alloc/alloc/fn.__rust_dealloc.html
// It has the following Rust signature:
//   `unsafe fn __rust_dealloc(ptr: *mut u8, size: usize, align: usize)`
// This low-level function is called by std::alloc:dealloc, and its
// implementation is provided by the compiler backend, so we need to provide an
// implementation for it to prevent verification failure due to missing function
// definition.
// For safety, refer to the documentation of GlobalAlloc::dealloc:
// https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html#tymethod.dealloc
void __rust_dealloc(uint8_t *ptr, size_t size, size_t align)
{
    // Note: we appear to do nothing with `align`
    // TODO: https://github.com/model-checking/kani/issues/1168
    __CPROVER_assert(__CPROVER_OBJECT_SIZE(ptr) == size,
                     "rust_dealloc must be called on an object whose allocated size matches its layout");
    __CPROVER_assume(__CPROVER_OBJECT_SIZE(ptr) == size);
    free(ptr);
}

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
    __CPROVER_assert(ptr != 0, "rust_realloc must be called with a non-null pointer");
    __CPROVER_assume(ptr != 0);

    // Passing a new_size of 0 is undefined behavior
    __CPROVER_assert(new_size > 0, "rust_realloc must be called with a size greater than 0");
    __CPROVER_assume(new_size > 0);

    uint8_t *result = malloc(new_size);
    if (result) {
        size_t bytes_to_copy = new_size < old_size ? new_size : old_size;
        memcpy(result, ptr, bytes_to_copy);
        free(ptr);
    }

    return result;
}
