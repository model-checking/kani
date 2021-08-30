// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

extern crate libc;

use self::libc::{c_uint, size_t};

// Abstraction which implements Vec operations using CBMC C primitives by only
// implementing skeleton functions in Rust and exporting all core functionality
// to C. 

extern "C" {
    fn vec_new() -> *mut c_vec;
    fn vec_push(ptr: *mut c_vec, elem: c_uint);
    fn vec_cap(ptr: *mut c_vec) -> c_uint;
    fn vec_len(ptr: *mut c_vec) -> c_uint;
    fn vec_with_capacity(cap: size_t) -> *mut c_vec;
    fn vec_pop(ptr: *mut c_vec) -> c_uint;
    fn vec_append(ptr1: *mut c_vec, ptr2: *mut c_vec);
    fn vec_insert(ptr: *mut c_vec, index: size_t, elem: c_uint);
}

// A c_vec consists of a pointer to memory, the capacity of the allocation and 
// the length of the vector to be verified. This struture is used across
// the exchange over the FFI boundary.
#[repr(C)]
pub struct c_vec {
    mem: *mut c_uint,
    len: size_t,
    capacity: size_t
}

// The Vec representation in Rust only tracks the pointer to the c_vec structure.
pub struct Vec {
    ptr: *mut c_vec,
}

impl Vec {
    pub fn ptr(&mut self) -> *mut c_vec {
        return self.ptr;
    }

    pub fn new() -> Self {
        unsafe {
            Vec {
                ptr: vec_new(),
            }
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        unsafe {
            Vec {
                ptr: vec_with_capacity(cap),
            }
        }
    }

    pub fn push(&mut self, elem: c_uint) {
        unsafe {
            vec_push(self.ptr, elem);
        }
    }

    pub fn pop(&mut self) -> Option<c_uint> {
        if self.len() == 0 {
            None
        } else {
            unsafe {
                Some(vec_pop(self.ptr))
            }
        }
    }

    pub fn append(&mut self, other: &mut Self) {
        unsafe {
            vec_append(self.ptr, other.ptr());
        }
    }

    pub fn capacity(&self) -> usize {
        unsafe {
            vec_cap(self.ptr) as usize
        }
    }

    pub fn len(&self) -> usize {
        unsafe {
            vec_len(self.ptr) as usize
        }
    }
}

#[cfg(abs_type = "c-ffi")]
#[macro_export]
macro_rules! rmc_vec {
  ( $val:expr ; $count:expr ) =>
    ({
      let mut result = Vec::new();
      let mut i: usize = 0;
      while i < $count {
        result.push($val);
        i += 1;
      }
      result
    });
  ( $( $xs:expr ),* ) => {
    {
      let mut result = Vec::new();
      $(
        result.push($xs);
      )*
      result
    }
  };
}
