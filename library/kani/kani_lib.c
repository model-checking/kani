// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#include <stddef.h>
#include <stdint.h>

// Declare functions instead of importing more headers in order to avoid conflicting definitions.
// See https://github.com/model-checking/kani/issues/1774 for more details.
void  free(void *ptr);
void *memcpy(void *dst, const void *src, size_t n);
void *calloc(size_t nmemb, size_t size);

typedef __CPROVER_bool bool;

/// Mapping unit to `void` works for functions with no return type but not for
/// variables with type unit. We treat both uniformly by declaring an empty
/// struct type: `struct Unit {}` and a global variable `struct Unit VoidUnit`
/// returned by all void functions (both declared by the Kani compiler).
struct Unit;
extern struct Unit VoidUnit;

// `assert` then `assume`
#define __KANI_assert(cond, msg)            \
    do {                                    \
        bool __KANI_temp = (cond);          \
        __CPROVER_assert(__KANI_temp, msg); \
        __CPROVER_assume(__KANI_temp);      \
    } while (0)

// Check that the input is either a power of 2, or 0. Algorithm from Hackers Delight.
bool __KANI_is_nonzero_power_of_two(size_t i) { return (i != 0) && (i & (i - 1)) == 0; }

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
    __KANI_assert(size > 0, "__rust_alloc must be called with a size greater than 0");
    // TODO: Ensure we are doing the right thing with align
    // https://github.com/model-checking/kani/issues/1168
    __KANI_assert(__KANI_is_nonzero_power_of_two(align), "Alignment is power of two");
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
    __KANI_assert(size > 0, "__rust_alloc_zeroed must be called with a size greater than 0");
    // TODO: Ensure we are doing the right thing with align
    // https://github.com/model-checking/kani/issues/1168
    __KANI_assert(__KANI_is_nonzero_power_of_two(align), "Alignment is power of two");
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
struct Unit __rust_dealloc(uint8_t *ptr, size_t size, size_t align)
{
    // TODO: Ensure we are doing the right thing with align
    // https://github.com/model-checking/kani/issues/1168
    __KANI_assert(__KANI_is_nonzero_power_of_two(align), "Alignment is power of two");

    __KANI_assert(__CPROVER_OBJECT_SIZE(ptr) == size,
                  "rust_dealloc must be called on an object whose allocated size matches its layout");
    free(ptr);
    return VoidUnit;
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
    __KANI_assert(ptr != 0, "rust_realloc must be called with a non-null pointer");

    // Passing a new_size of 0 is undefined behavior
    __KANI_assert(new_size > 0, "rust_realloc must be called with a size greater than 0");

    // TODO: Ensure we are doing the right thing with align
    // https://github.com/model-checking/kani/issues/1168
    __KANI_assert(__KANI_is_nonzero_power_of_two(align), "Alignment is power of two");

    uint8_t *result = malloc(new_size);
    if (result) {
        size_t bytes_to_copy = new_size < old_size ? new_size : old_size;
        memcpy(result, ptr, bytes_to_copy);
        free(ptr);
    }

    return result;
}

size_t __KANI_pointer_object(uint8_t *ptr) {
    return __CPROVER_POINTER_OBJECT(ptr);
}

size_t __KANI_pointer_offset(uint8_t *ptr) {
    return __CPROVER_POINTER_OFFSET(ptr);
}
