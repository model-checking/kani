// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module uses shadow memory API to track memory initialization of raw pointers.
//!
//! Currently, memory initialization is tracked on per-byte basis, so each byte of memory pointed to
//! by raw pointers could be either initialized or uninitialized. Compiler automatically inserts
//! calls to `is_xxx_initialized` and `set_xxx_initialized` at appropriate locations to get or set
//! the initialization status of the memory pointed to. Padding bytes are always considered
//! uninitialized: type layout is determined at compile time and statically injected into the
//! program (see `Layout`).

// Definitions in this module are not meant to be visible to the end user, only the compiler.
#![allow(dead_code)]

use crate::shadow::ShadowMem;

/// Bytewise mask, representing which bytes of a type are data and which are padding.
/// For example, for a type like this:
/// ```
/// #[repr(C)]
/// struct Foo {
///     a: u16,
///     b: u8,
/// }
/// ```
/// the layout would be [true, true, true, false];
type Layout<const N: usize> = [bool; N];

/// Global shadow memory object for tracking memory initialization.
#[rustc_diagnostic_item = "KaniMemInitShadowMem"]
static mut MEM_INIT_SHADOW_MEM: ShadowMem<bool> = ShadowMem::new(false);

/// Get initialization state of `len` items laid out according to the `layout` starting at address `ptr`.
#[rustc_diagnostic_item = "KaniIsUnitPtrInitialized"]
fn is_unit_ptr_initialized<const N: usize>(ptr: *const (), layout: Layout<N>, len: usize) -> bool {
    let mut count: usize = 0;
    while count < len {
        let mut offset: usize = 0;
        while offset < N {
            unsafe {
                if layout[offset]
                    && !MEM_INIT_SHADOW_MEM.get((ptr as *const u8).add(count * N + offset))
                {
                    return false;
                }
                offset += 1;
            }
        }
        count += 1;
    }
    true
}

/// Set initialization state to `value` for `len` items laid out according to the `layout` starting at address `ptr`.
#[rustc_diagnostic_item = "KaniSetUnitPtrInitialized"]
fn set_unit_ptr_initialized<const N: usize>(
    ptr: *const (),
    layout: Layout<N>,
    len: usize,
    value: bool,
) {
    let mut count: usize = 0;
    while count < len {
        let mut offset: usize = 0;
        while offset < N {
            unsafe {
                MEM_INIT_SHADOW_MEM
                    .set((ptr as *const u8).add(count * N + offset), value && layout[offset]);
            }
            offset += 1;
        }
        count += 1;
    }
}

/// Get initialization state of `len` items laid out according to the `layout` starting at address `ptr`.
#[rustc_diagnostic_item = "KaniIsPtrInitialized"]
fn is_ptr_initialized<const N: usize, T>(ptr: *const T, layout: Layout<N>, len: usize) -> bool {
    let (ptr, _) = ptr.to_raw_parts();
    is_unit_ptr_initialized(ptr, layout, len)
}

/// Set initialization state to `value` for `len` items laid out according to the `layout` starting at address `ptr`.
#[rustc_diagnostic_item = "KaniSetPtrInitialized"]
fn set_ptr_initialized<const N: usize, T>(
    ptr: *const T,
    layout: Layout<N>,
    len: usize,
    value: bool,
) {
    let (ptr, _) = ptr.to_raw_parts();
    set_unit_ptr_initialized(ptr, layout, len, value);
}

/// Get initialization state of the slice, items of which are laid out according to the `layout` starting at address `ptr`.
#[rustc_diagnostic_item = "KaniIsSlicePtrInitialized"]
fn is_slice_ptr_initialized<const N: usize, T>(ptr: *const [T], layout: Layout<N>) -> bool {
    let (ptr, len) = ptr.to_raw_parts();
    is_unit_ptr_initialized(ptr, layout, len)
}

/// Set initialization state of the slice, items of which are laid out according to the `layout` starting at address `ptr` to `value`.
#[rustc_diagnostic_item = "KaniSetSlicePtrInitialized"]
fn set_slice_ptr_initialized<const N: usize, T>(ptr: *const [T], layout: Layout<N>, value: bool) {
    let (ptr, len) = ptr.to_raw_parts();
    set_unit_ptr_initialized(ptr, layout, len, value);
}

/// Get initialization state of the string slice, items of which are laid out according to the `layout` starting at address `ptr`.
#[rustc_diagnostic_item = "KaniIsStrPtrInitialized"]
fn is_str_ptr_initialized<const N: usize>(ptr: *const str, layout: Layout<N>) -> bool {
    let (ptr, len) = ptr.to_raw_parts();
    is_unit_ptr_initialized(ptr, layout, len)
}

/// Set initialization state of the string slice, items of which are laid out according to the `layout` starting at address `ptr` to `value`.
#[rustc_diagnostic_item = "KaniSetStrPtrInitialized"]
fn set_str_ptr_initialized<const N: usize>(ptr: *const str, layout: Layout<N>, value: bool) {
    let (ptr, len) = ptr.to_raw_parts();
    set_unit_ptr_initialized(ptr, layout, len, value);
}
