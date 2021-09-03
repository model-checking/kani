// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

mod utils;
use utils::libc::{int16_t, size_t, uint16_t, uint32_t};

use std::marker::PhantomData;

// CVec is an abstraction of the Vector library which is implemented as a Rust-based
// frontend and a C based backend. All public facing methods here are implemented
// as wrappers around FFI functions which call into methods implemented in C. There
// were multiple reasons as to why this abstractions was conceived:
//
// 1. Reduce the cost of translation: RMC translates Rust code into equivalent
// gotoC representation which is used for verification with CBMC. However, this
// might introduce additional overhead due to nesting of calls, monomorphization,
// handling generics, etc. CVec gets around that issue by making direct calls
// to the C implementation. This is usually hard to reason about for most other
// frameworks since they are unable to handle unsafe code. But because of the
// way RMC works, it is almost zero cost to achieve this.
//
// 2. Leverage CBMC primitives: Some CBMC primitives cannot yet be correctly
// translated/handled by RMC, for instance: __CPROVER_constant_infinity_uint. If
// there are improvements in CBMC, a quick way to test their applicability in
// RMC would be to work with this abstraction and perform experiments.

// A c_vec consists of a pointer to allocated memory, the capacity of the allocation and
// the length of the vector to be used in verification. This structure is also
// defined in c_vec and is used across the FFI boundary.
//
// An important callout to make here is that this structure is currently only defined
// to work with a Vec<u32>. This code was meant to experiment with and demonstrate
// the ability to work with CBMCs C interface.
#[repr(C)]
pub struct c_vec {
    mem: *mut uint32_t,
    len: size_t,
    capacity: size_t,
}

// All of the functions below call into implementations defined in vec.c
//
// We could also move to a more polished definition which is defined in a .h header
// file which is what this interface would need to care about. For now, the
// definition and the implementation reside in vec.c.
//
// Although these are defined manually, it might be worthwhile to look at projects
// such as cbindgen which can generate automatically generate headers for Rust
// code which exposes a public interface. For instance, we could define generic
// Vector representations and have the framework generate headers for us which we
// can then implement. In that case, we would have to implement the C backend in
// such a way that it can handle types of arbitrary sizes by casting memory blocks
// and Vector elements and treating them as such.
// Reference: https://github.com/eqrion/cbindgen
extern "C" {
    // Returns pointer to a new c_vec structure. The default capacity of the allocated
    // vec is (1073741824 / sizeof(u32)) at the maximum.
    fn vec_new() -> *mut c_vec;

    // Returns pointer to a new c_vec structure. The capacity is provided as an
    // argument.
    fn vec_with_capacity(cap: size_t) -> *mut c_vec;

    // Pushes a new elements to the Vector. If there is not enough space to allocate
    // the element, the Vector will resize itself.
    fn vec_push(ptr: *mut c_vec, elem: uint32_t);

    // Pop an element out of the Vector. The wrapper function contains a check
    // to ensure that we are not popping a value off of an empty Vector.
    fn vec_pop(ptr: *mut c_vec) -> uint32_t;

    // Returns the current capacity of allocation.
    fn vec_cap(ptr: *mut c_vec) -> size_t;

    // Returns the length of the Vector
    fn vec_len(ptr: *mut c_vec) -> size_t;

    // Append Vector represented by ptr2 to ptr1.
    fn vec_append(ptr1: *mut c_vec, ptr2: *mut c_vec);

    // Grow the allocated vector in size such that it accomodates atleast
    // additional elements. This is similar in behavior to the implementation of
    // the Rust Standard Library. Please refer to vec.c for more details.
    fn vec_sized_grow(ptr: *mut c_vec, additional: size_t);
}

// The Vec interface which is exposed to the user only tracks the pointer to the
// low-level c_vec structure. All methods defined on this structure act as wrappers
// and call into the C implementation.
//
// The implementation is currently not generic over the contained type.
pub struct Vec<T> {
    ptr: *mut c_vec,
    _marker: PhantomData<T>,
}

// Wrapper methods which ensure that consumer code does not have to make calls
// to unsafe C-FFI functions.
impl<T> Vec<T> {
    pub fn ptr(&mut self) -> *mut c_vec {
        return self.ptr;
    }

    pub fn new() -> Self {
        unsafe { Vec { ptr: vec_new(), _marker: Default::default() } }
    }

    pub fn with_capacity(cap: usize) -> Self {
        unsafe { Vec { ptr: vec_with_capacity(cap), _marker: Default::default() } }
    }

    pub fn push(&mut self, elem: uint32_t) {
        unsafe {
            vec_push(self.ptr, elem);
        }
    }

    // Check if the length of the Vector is 0, in which case we return a None.
    // Otherwise, we make a call to the vec_pop() function and wrap the result around
    // a Some.
    pub fn pop(&mut self) -> Option<uint32_t> {
        if self.len() == 0 { None } else { unsafe { Some(vec_pop(self.ptr)) } }
    }

    pub fn append(&mut self, other: &mut Self) {
        unsafe {
            vec_append(self.ptr, other.ptr());
        }
    }

    pub fn capacity(&self) -> usize {
        unsafe { vec_cap(self.ptr) as usize }
    }

    pub fn len(&self) -> usize {
        unsafe { vec_len(self.ptr) as usize }
    }

    pub fn reserve(&mut self, additional: usize) {
        unsafe {
            vec_sized_grow(self.ptr, additional);
        }
    }
}

#[cfg(abs_type = "c-ffi")]
#[macro_export]
macro_rules! rmc_vec {
  ( $val:expr ; $count:expr ) =>
    ({
      let mut result = Vec::with_capacity($count);
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
