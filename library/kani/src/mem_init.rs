// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module provides instrumentation for tracking memory initialization of raw pointers.
//!
//! Currently, memory initialization is tracked on per-byte basis, so each byte of memory pointed to
//! by raw pointers could be either initialized or uninitialized. Padding bytes are always
//! considered uninitialized when read as data bytes. Each type has a type layout to specify which
//! bytes are considered to be data and which -- padding. This is determined at compile time and
//! statically injected into the program (see `Layout`).
//!
//! Compiler automatically inserts calls to `is_xxx_initialized` and `set_xxx_initialized` at
//! appropriate locations to get or set the initialization status of the memory pointed to.
//!
//! Note that for each harness, tracked object and tracked offset are chosen non-deterministically,
//! so calls to `is_xxx_initialized` should be only used in assertion contexts.

// Definitions in this module are not meant to be visible to the end user, only the compiler.
#![allow(dead_code)]

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
type Layout<const LAYOUT_SIZE: usize> = [bool; LAYOUT_SIZE];

/// Currently tracked non-deterministically chosen memory initialization state.
struct MemoryInitializationState {
    pub tracked_object_id: usize,
    pub tracked_offset: usize,
    pub value: bool,
}

impl MemoryInitializationState {
    /// This is a dummy initialization function -- the values will be eventually overwritten by a
    /// call to `initialize_memory_initialization_state`.
    pub const fn new() -> Self {
        Self { tracked_object_id: 0, tracked_offset: 0, value: false }
    }

    /// Return currently tracked memory initialization state if `ptr` points to the currently
    /// tracked object and the tracked offset lies within `LAYOUT_SIZE` bytes of `ptr`. Return
    /// `true` otherwise.
    ///
    /// Such definition is necessary since both tracked object and tracked offset are chosen
    /// non-deterministically.
    #[kanitool::skip_ptr_checks]
    pub fn get<const LAYOUT_SIZE: usize>(
        &mut self,
        ptr: *const u8,
        layout: Layout<LAYOUT_SIZE>,
    ) -> bool {
        let obj = crate::mem::pointer_object(ptr);
        let offset = crate::mem::pointer_offset(ptr);
        if self.tracked_object_id == obj
            && self.tracked_offset >= offset
            && self.tracked_offset < offset + LAYOUT_SIZE
        {
            !layout[self.tracked_offset - offset] || self.value
        } else {
            true
        }
    }

    /// Set currently tracked memory initialization state if `ptr` points to the currently tracked
    /// object and the tracked offset lies within `LAYOUT_SIZE` bytes of `ptr`. Do nothing
    /// otherwise.
    ///
    /// Such definition is necessary since both tracked object and tracked offset are chosen
    /// non-deterministically.
    #[kanitool::skip_ptr_checks]
    pub fn set<const LAYOUT_SIZE: usize>(
        &mut self,
        ptr: *const u8,
        layout: Layout<LAYOUT_SIZE>,
        value: bool,
    ) {
        let obj = crate::mem::pointer_object(ptr);
        let offset = crate::mem::pointer_offset(ptr);
        if self.tracked_object_id == obj
            && self.tracked_offset >= offset
            && self.tracked_offset < offset + LAYOUT_SIZE
        {
            self.value = layout[self.tracked_offset - offset] && value;
        }
    }

    /// Return currently tracked memory initialization state if `ptr` points to the currently
    /// tracked object and the tracked offset lies within `LAYOUT_SIZE * num_elts` bytes of `ptr`.
    /// Return `true` otherwise.
    ///
    /// Such definition is necessary since both tracked object and tracked offset are chosen
    /// non-deterministically.
    #[kanitool::skip_ptr_checks]
    pub fn get_slice<const LAYOUT_SIZE: usize>(
        &mut self,
        ptr: *const u8,
        layout: Layout<LAYOUT_SIZE>,
        num_elts: usize,
    ) -> bool {
        let obj = crate::mem::pointer_object(ptr);
        let offset = crate::mem::pointer_offset(ptr);
        if self.tracked_object_id == obj
            && self.tracked_offset >= offset
            && self.tracked_offset < offset + num_elts * LAYOUT_SIZE
        {
            !layout[(self.tracked_offset - offset) % LAYOUT_SIZE] || self.value
        } else {
            true
        }
    }

    /// Set currently tracked memory initialization state if `ptr` points to the currently tracked
    /// object and the tracked offset lies within `LAYOUT_SIZE * num_elts` bytes of `ptr`. Do
    /// nothing otherwise.
    ///
    /// Such definition is necessary since both tracked object and tracked offset are chosen
    /// non-deterministically.
    #[kanitool::skip_ptr_checks]
    pub fn set_slice<const LAYOUT_SIZE: usize>(
        &mut self,
        ptr: *const u8,
        layout: Layout<LAYOUT_SIZE>,
        num_elts: usize,
        value: bool,
    ) {
        let obj = crate::mem::pointer_object(ptr);
        let offset = crate::mem::pointer_offset(ptr);
        if self.tracked_object_id == obj
            && self.tracked_offset >= offset
            && self.tracked_offset < offset + num_elts * LAYOUT_SIZE
        {
            self.value = layout[(self.tracked_offset - offset) % LAYOUT_SIZE] && value;
        }
    }
}

/// Global object for tracking memory initialization state.
#[rustc_diagnostic_item = "KaniMemoryInitializationState"]
static mut MEM_INIT_STATE: MemoryInitializationState = MemoryInitializationState::new();

/// Set tracked object and tracked offset to a non-deterministic value.
#[kanitool::skip_ptr_checks]
#[rustc_diagnostic_item = "KaniInitializeMemoryInitializationState"]
fn initialize_memory_initialization_state() {
    unsafe {
        MEM_INIT_STATE.tracked_object_id = crate::any();
        MEM_INIT_STATE.tracked_offset = crate::any();
        MEM_INIT_STATE.value = false;
    }
}

/// Get initialization state of `num_elts` items laid out according to the `layout` starting at address `ptr`.
#[kanitool::skip_ptr_checks]
#[rustc_diagnostic_item = "KaniIsPtrInitialized"]
fn is_ptr_initialized<const LAYOUT_SIZE: usize, T>(
    ptr: *const T,
    layout: Layout<LAYOUT_SIZE>,
) -> bool {
    if LAYOUT_SIZE == 0 {
        return true;
    }
    let (ptr, _) = ptr.to_raw_parts();
    unsafe { MEM_INIT_STATE.get(ptr as *const u8, layout) }
}

/// Set initialization state to `value` for `num_elts` items laid out according to the `layout` starting at address `ptr`.
#[kanitool::skip_ptr_checks]
#[rustc_diagnostic_item = "KaniSetPtrInitialized"]
fn set_ptr_initialized<const LAYOUT_SIZE: usize, T>(
    ptr: *const T,
    layout: Layout<LAYOUT_SIZE>,
    value: bool,
) {
    if LAYOUT_SIZE == 0 {
        return;
    }
    let (ptr, _) = ptr.to_raw_parts();
    unsafe {
        MEM_INIT_STATE.set(ptr as *const u8, layout, value);
    }
}

/// Get initialization state of `num_elts` items laid out according to the `layout` starting at address `ptr`.
#[kanitool::skip_ptr_checks]
#[rustc_diagnostic_item = "KaniIsSliceChunkPtrInitialized"]
fn is_slice_chunk_ptr_initialized<const LAYOUT_SIZE: usize, T>(
    ptr: *const T,
    layout: Layout<LAYOUT_SIZE>,
    num_elts: usize,
) -> bool {
    if LAYOUT_SIZE == 0 {
        return true;
    }
    let (ptr, _) = ptr.to_raw_parts();
    unsafe { MEM_INIT_STATE.get_slice(ptr as *const u8, layout, num_elts) }
}

/// Set initialization state to `value` for `num_elts` items laid out according to the `layout` starting at address `ptr`.
#[kanitool::skip_ptr_checks]
#[rustc_diagnostic_item = "KaniSetSliceChunkPtrInitialized"]
fn set_slice_chunk_ptr_initialized<const LAYOUT_SIZE: usize, T>(
    ptr: *const T,
    layout: Layout<LAYOUT_SIZE>,
    num_elts: usize,
    value: bool,
) {
    if LAYOUT_SIZE == 0 {
        return;
    }
    let (ptr, _) = ptr.to_raw_parts();
    unsafe {
        MEM_INIT_STATE.set_slice(ptr as *const u8, layout, num_elts, value);
    }
}

/// Get initialization state of the slice, items of which are laid out according to the `layout` starting at address `ptr`.
#[kanitool::skip_ptr_checks]
#[rustc_diagnostic_item = "KaniIsSlicePtrInitialized"]
fn is_slice_ptr_initialized<const LAYOUT_SIZE: usize, T>(
    ptr: *const [T],
    layout: Layout<LAYOUT_SIZE>,
) -> bool {
    if LAYOUT_SIZE == 0 {
        return true;
    }
    let (ptr, num_elts) = ptr.to_raw_parts();
    unsafe { MEM_INIT_STATE.get_slice(ptr as *const u8, layout, num_elts) }
}

/// Set initialization state of the slice, items of which are laid out according to the `layout` starting at address `ptr` to `value`.
#[kanitool::skip_ptr_checks]
#[rustc_diagnostic_item = "KaniSetSlicePtrInitialized"]
fn set_slice_ptr_initialized<const LAYOUT_SIZE: usize, T>(
    ptr: *const [T],
    layout: Layout<LAYOUT_SIZE>,
    value: bool,
) {
    if LAYOUT_SIZE == 0 {
        return;
    }
    let (ptr, num_elts) = ptr.to_raw_parts();
    unsafe {
        MEM_INIT_STATE.set_slice(ptr as *const u8, layout, num_elts, value);
    }
}

/// Get initialization state of the string slice, items of which are laid out according to the `layout` starting at address `ptr`.
#[kanitool::skip_ptr_checks]
#[rustc_diagnostic_item = "KaniIsStrPtrInitialized"]
fn is_str_ptr_initialized<const LAYOUT_SIZE: usize>(
    ptr: *const str,
    layout: Layout<LAYOUT_SIZE>,
) -> bool {
    if LAYOUT_SIZE == 0 {
        return true;
    }
    let (ptr, num_elts) = ptr.to_raw_parts();
    unsafe { MEM_INIT_STATE.get_slice(ptr as *const u8, layout, num_elts) }
}

/// Set initialization state of the string slice, items of which are laid out according to the `layout` starting at address `ptr` to `value`.
#[kanitool::skip_ptr_checks]
#[rustc_diagnostic_item = "KaniSetStrPtrInitialized"]
fn set_str_ptr_initialized<const LAYOUT_SIZE: usize>(
    ptr: *const str,
    layout: Layout<LAYOUT_SIZE>,
    value: bool,
) {
    if LAYOUT_SIZE == 0 {
        return;
    }
    let (ptr, num_elts) = ptr.to_raw_parts();
    unsafe {
        MEM_INIT_STATE.set_slice(ptr as *const u8, layout, num_elts, value);
    }
}
