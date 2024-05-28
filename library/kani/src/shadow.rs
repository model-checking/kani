// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

extern "C" {
    pub fn __KANI_pointer_object(ptr: *const u8) -> usize;
    pub fn __KANI_pointer_offset(ptr: *const u8) -> usize;
}

pub struct ShadowMem {
    is_init: [[bool; 64]; 1024],
}

impl ShadowMem {
    pub const fn new() -> Self {
        Self { is_init: [[false; 64]; 1024] }
    }

    pub fn is_init(&self, ptr: *const u8) -> bool {
        read(&self.is_init, ptr)
    }

    pub fn set_init(&mut self, ptr: *const u8, init: bool) {
        write(&mut self.is_init, ptr, init);
    }
}

pub fn read(sm: &[[bool; 64]; 1024], ptr: *const u8) -> bool {
    let obj = unsafe { __KANI_pointer_object(ptr) };
    let offset = unsafe { __KANI_pointer_offset(ptr) };
    sm[obj][offset]
}

pub fn write(sm: &mut [[bool; 64]; 1024], ptr: *const u8, val: bool) {
    let obj = unsafe { __KANI_pointer_object(ptr) };
    let offset = unsafe { __KANI_pointer_offset(ptr) };
    sm[obj][offset] = val;
}
