// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

extern crate libc;
use self::libc::{c_int, c_uint};

// HashSet abstraction. Currently only implements the idea for a HashSet<u32> 
// as an experiment.
//
// RMC currently does not support unbounded structures and arrays:
// Tracking issue: https://github.com/model-checking/rmc/issues/311
//
// RMC might also be blocked due to failure to correctly translate infinity 
// in CBMC. 
// Tracking issue: https://github.com/diffblue/cbmc/issues/6261

extern "C" {
    fn hashset_new() -> *mut c_hashset;
    fn hashset_insert(ptr: *mut c_hashset, value: c_uint) -> c_uint;
    fn hashset_contains(ptr: *mut c_hashset, value: c_uint) -> c_uint;
    fn hashset_remove(ptr: *mut c_hashset, value: c_uint) -> c_uint;
}

#[repr(C)]
pub struct c_hashset {
    domain: [c_int; u32::MAX as usize],
    counter: c_uint
}

pub struct HashSet {
    ptr: *mut c_hashset,
}

// Currently only implemented for c_uint = u32
impl HashSet {
    pub fn new() -> Self {
        unsafe {
            HashSet {
                ptr: hashset_new(),
            }
        }
    }

    pub fn insert(&mut self, value: c_uint) -> bool {
        unsafe {
            hashset_insert(self.ptr, value) != 0
        }
    }

    pub fn contains(&self, value: &c_uint) -> bool {
        unsafe {
            hashset_contains(self.ptr, *value) != 0
        }
    }

    pub fn remove(&mut self, value: c_uint) -> bool {
        unsafe {
            hashset_remove(self.ptr, value) != 0
        }
    }
}
