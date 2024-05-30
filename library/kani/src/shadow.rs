// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

extern "C" {
    pub fn __KANI_pointer_object(ptr: *const u8) -> usize;
    pub fn __KANI_pointer_offset(ptr: *const u8) -> usize;
}

const MAX_NUM_OBJECTS: usize = 1024;
const MAX_OBJECT_SIZE: usize = 64;

pub struct ShadowMem {
    is_init: [[bool; MAX_OBJECT_SIZE]; MAX_NUM_OBJECTS],
}

impl ShadowMem {
    pub const fn new() -> Self {
        Self { is_init: [[false; MAX_OBJECT_SIZE]; MAX_NUM_OBJECTS] }
    }

    /// # Safety
    ///
    /// `ptr` must be valid
    pub unsafe fn is_init(&self, ptr: *const u8) -> bool {
        unsafe { read(&self.is_init, ptr) }
    }

    /// # Safety
    ///
    /// `ptr` must be valid
    pub unsafe fn set_init(&mut self, ptr: *const u8, init: bool) {
        unsafe { write(&mut self.is_init, ptr, init) };
    }
}

const MAX_NUM_OBJECTS_ASSERT_MSG: &str = "The number of objects exceeds the maximum number supported by Kani's shadow memory model (1024)";
const MAX_OBJECT_SIZE_ASSERT_MSG: &str =
    "The object size exceeds the maximum size supported by Kani's shadow memory model (64)";

/// # Safety
///
/// `ptr` must be valid
pub unsafe fn read(sm: &[[bool; MAX_OBJECT_SIZE]; MAX_NUM_OBJECTS], ptr: *const u8) -> bool {
    let obj = unsafe { __KANI_pointer_object(ptr) };
    let offset = unsafe { __KANI_pointer_offset(ptr) };
    crate::assert(obj < MAX_NUM_OBJECTS, MAX_NUM_OBJECTS_ASSERT_MSG);
    crate::assert(offset < MAX_OBJECT_SIZE, MAX_OBJECT_SIZE_ASSERT_MSG);
    sm[obj][offset]
}

/// # Safety
///
/// `ptr` must be valid
pub unsafe fn write(
    sm: &mut [[bool; MAX_OBJECT_SIZE]; MAX_NUM_OBJECTS],
    ptr: *const u8,
    val: bool,
) {
    let obj = unsafe { __KANI_pointer_object(ptr) };
    let offset = unsafe { __KANI_pointer_offset(ptr) };
    crate::assert(obj < MAX_NUM_OBJECTS, MAX_NUM_OBJECTS_ASSERT_MSG);
    crate::assert(offset < MAX_OBJECT_SIZE, MAX_OBJECT_SIZE_ASSERT_MSG);
    sm[obj][offset] = val;
}
