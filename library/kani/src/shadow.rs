// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module contains an API for shadow memory.
//! Shadow memory is a mechanism by which we can store metadata on memory
//! locations, e.g. whether a memory location is initialized.
//!
//! The main data structure provided by this module is the `ShadowMem` struct,
//! which allows us to store metadata on a given memory location.
//!
//! # Example
//!
//! ```
//! use kani::shadow::ShadowMem;
//! use std::alloc::{alloc, Layout};
//!
//! let mut sm = ShadowMem::new(false);
//!
//! unsafe {
//!     let ptr = alloc(Layout::new::<u8>());
//!     // assert the memory location is not initialized
//!     assert!(!sm.get(ptr));
//!     // write to the memory location
//!     *ptr = 42;
//!     // update the shadow memory to indicate that this location is now initialized
//!     sm.set(ptr, true);
//! }
//! ```

const MAX_NUM_OBJECTS: usize = 1024;
const MAX_OBJECT_SIZE: usize = 64;

const MAX_NUM_OBJECTS_ASSERT_MSG: &str = "The number of objects exceeds the maximum number supported by Kani's shadow memory model (1024)";
const MAX_OBJECT_SIZE_ASSERT_MSG: &str =
    "The object size exceeds the maximum size supported by Kani's shadow memory model (64)";

/// A shadow memory data structure that contains a two-dimensional array of a
/// generic type `T`.
/// Each element of the outer array represents an object, and each element of
/// the inner array represents a byte in the object.
pub struct ShadowMem<T: Copy> {
    mem: [[T; MAX_OBJECT_SIZE]; MAX_NUM_OBJECTS],
}

impl<T: Copy> ShadowMem<T> {
    /// Create a new shadow memory instance initialized with the given value
    #[crate::unstable(
        feature = "ghost-state",
        issue = 3184,
        reason = "experimental ghost state/shadow memory API"
    )]
    pub const fn new(val: T) -> Self {
        Self { mem: [[val; MAX_OBJECT_SIZE]; MAX_NUM_OBJECTS] }
    }

    /// Get the shadow memory value of the given pointer
    #[crate::unstable(
        feature = "ghost-state",
        issue = 3184,
        reason = "experimental ghost state/shadow memory API"
    )]
    pub fn get<U>(&self, ptr: *const U) -> T {
        let obj = crate::mem::pointer_object(ptr);
        let offset = crate::mem::pointer_offset(ptr);
        crate::assert(obj < MAX_NUM_OBJECTS, MAX_NUM_OBJECTS_ASSERT_MSG);
        crate::assert(offset < MAX_OBJECT_SIZE, MAX_OBJECT_SIZE_ASSERT_MSG);
        self.mem[obj][offset]
    }

    /// Set the shadow memory value of the given pointer
    #[crate::unstable(
        feature = "ghost-state",
        issue = 3184,
        reason = "experimental ghost state/shadow memory API"
    )]
    pub fn set<U>(&mut self, ptr: *const U, val: T) {
        let obj = crate::mem::pointer_object(ptr);
        let offset = crate::mem::pointer_offset(ptr);
        crate::assert(obj < MAX_NUM_OBJECTS, MAX_NUM_OBJECTS_ASSERT_MSG);
        crate::assert(offset < MAX_OBJECT_SIZE, MAX_OBJECT_SIZE_ASSERT_MSG);
        self.mem[obj][offset] = val;
    }
}

/// Global shadow memory object for tracking memory initialization.
#[rustc_diagnostic_item = "KaniMemInitSM"]
static mut __KANI_MEM_INIT_SM: ShadowMem<bool> = ShadowMem::new(false);

// Get initialization setate of `n` items laid out according to the `layout` starting at address `ptr`.
#[rustc_diagnostic_item = "KaniMemInitSMGetInner"]
fn __kani_mem_init_sm_get_inner<const N: usize>(
    ptr: *const (),
    layout: [bool; N],
    n: usize,
) -> bool {
    let mut count: usize = 0;
    while count < n {
        let mut offset: usize = 0;
        while offset < N {
            unsafe {
                if layout[offset]
                    && !__KANI_MEM_INIT_SM.get((ptr as *const u8).add(count * N + offset))
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

// Set initialization setate to `value` for `n` items laid out according to the `layout` starting at address `ptr`.
#[rustc_diagnostic_item = "KaniMemInitSMSetInner"]
fn __kani_mem_init_sm_set_inner<const N: usize>(
    ptr: *const (),
    layout: [bool; N],
    n: usize,
    value: bool,
) {
    let mut count: usize = 0;
    while count < n {
        let mut offset: usize = 0;
        while offset < N {
            unsafe {
                __KANI_MEM_INIT_SM
                    .set((ptr as *const u8).add(count * N + offset), value && layout[offset]);
            }
            offset += 1;
        }
        count += 1;
    }
}

#[rustc_diagnostic_item = "KaniMemInitSMGet"]
pub fn __kani_mem_init_sm_get<const N: usize, T: Sized>(
    ptr: *const T,
    layout: [bool; N],
    n: usize,
) -> bool {
    let (ptr, _) = ptr.to_raw_parts();
    __kani_mem_init_sm_get_inner(ptr, layout, n)
}

#[rustc_diagnostic_item = "KaniMemInitSMSet"]
pub fn __kani_mem_init_sm_set<const N: usize, T: Sized>(
    ptr: *const T,
    layout: [bool; N],
    n: usize,
    value: bool,
) {
    let (ptr, _) = ptr.to_raw_parts();
    __kani_mem_init_sm_set_inner(ptr, layout, n, value);
}

// This method should only be called if T is known to be a slice.
#[rustc_diagnostic_item = "KaniMemInitSMGetSlice"]
pub fn __kani_mem_init_sm_get_slice<const N: usize, T: ?Sized>(
    ptr: *const T,
    layout: [bool; N],
) -> bool {
    let (ptr, meta) = ptr.to_raw_parts();
    let n: usize = unsafe { std::mem::transmute_copy(&meta) };
    // The pointee type is a slice, more than `n` objects can be accessed.
    __kani_mem_init_sm_get_inner(ptr, layout, n)
}

// This method should only be called if T is known to be a slice.
#[rustc_diagnostic_item = "KaniMemInitSMSetSlice"]
pub fn __kani_mem_init_sm_set_slice<const N: usize, T: ?Sized>(
    ptr: *const T,
    layout: [bool; N],
    value: bool,
) {
    let (ptr, meta) = ptr.to_raw_parts();
    let n: usize = unsafe { std::mem::transmute_copy(&meta) };
    // The pointee type is a slice, more than `n` objects can be accessed.
    __kani_mem_init_sm_set_inner(ptr, layout, n, value);
}
