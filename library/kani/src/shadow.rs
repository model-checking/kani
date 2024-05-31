// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

const MAX_NUM_OBJECTS: usize = 1024;
const MAX_OBJECT_SIZE: usize = 64;

const MAX_NUM_OBJECTS_ASSERT_MSG: &str = "The number of objects exceeds the maximum number supported by Kani's shadow memory model (1024)";
const MAX_OBJECT_SIZE_ASSERT_MSG: &str =
    "The object size exceeds the maximum size supported by Kani's shadow memory model (64)";

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
        let obj = crate::mem::pointer_object(ptr);
        let offset = crate::mem::pointer_offset(ptr);
        crate::assert(obj < MAX_NUM_OBJECTS, MAX_NUM_OBJECTS_ASSERT_MSG);
        crate::assert(offset < MAX_OBJECT_SIZE, MAX_OBJECT_SIZE_ASSERT_MSG);
        self.is_init[obj][offset]
    }

    /// # Safety
    ///
    /// `ptr` must be valid
    pub unsafe fn set_init(&mut self, ptr: *const u8, init: bool) {
        let obj = crate::mem::pointer_object(ptr);
        let offset = crate::mem::pointer_offset(ptr);
        crate::assert(obj < MAX_NUM_OBJECTS, MAX_NUM_OBJECTS_ASSERT_MSG);
        crate::assert(offset < MAX_OBJECT_SIZE, MAX_OBJECT_SIZE_ASSERT_MSG);
        self.is_init[obj][offset] = init;
    }
}
