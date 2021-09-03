// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// NOTE: Code in this file and hashset.c is experimental and is meant to be a
// proof-of-concept implementation of the idea. It is unsound and might not work
// with all test cases. More details below.

// CHashSet is an abstraction of the HashSet library. This is also implemented as
// a Rust frontend and a C backend (similar to CVec). HashSets are hard to reason
// about for verification tools due to the nature of hash functions. To that end,
// this program tries to implement a HashSet for u16 values.
//
// A typical hashset is defined using two main components:
//
// 1. Hash function which maps the values in the input domain to a set of values
// in an output domain. Ideally, the output domain is larger than the input domain
// to ensure that there are special values such as the SENTINEL value which cannot
// be generated through the hashing function. Hash functions are 1:1 injections -
// for a certain input, they will deterministically generate the same output value
// and that no other input will generate that hash value.
// For our case, we have implemented this idea for u16. This implies that
// the input domain is <0 .. u16::MAX>. The output domain is chosen to be
// i16 <-i16::MAX .. i16::MAX>. We can theoretically choose any output domain
// which can provide us with a special value such that it does not lie in the range
// of the hash function.
//
// 2. HashSets also have a map which allow us to check the existence of an element
// in amortized constant O(1) time. They use the hashed value as the key into the
// map to check if a truth value is present. For implementing HashMaps however, we
// need to store the mapped value at the hashed location.
//
// We implement this idea in hashset.c, please refer to that file for implementation
// specifics.

// c_hashset consists of a pointer to the memory which tracks the hashset allocation
// in the C backend. This is used to exchange information over the FFI boundary.
//
// In theory, this can also be implemented purely in Rust. We chose to implement
// using the C-FFI to leverage CBMC constructs.
//
// But it important to note here that RMC currently does not support unbounded
// structures anda arrays;
// Tracking issue: https://github.com/model-checking/rmc/issues/311
#[repr(C)]
pub struct c_hashset {
    domain: *mut int16_t,
}

// All of the functions below call into implementations defined in vec.c.
//
// For other related future work on how this interface can be automatically
// generated and made cleaner, please refer c_vec.rs.
extern "C" {
    // Returns a pointer to a new c_hashset structure.
    fn hashset_new() -> *mut c_hashset;

    // Inserts a new value in the hashset. If the value is already present,
    // this function returns 0 else, returns 1.
    fn hashset_insert(ptr: *mut c_hashset, value: uint16_t) -> uint32_t;

    // Checks if the value is contained in the hashset. Returns 1 if present, 0
    // otherwise.
    fn hashset_contains(ptr: *mut c_hashset, value: uint16_t) -> uint32_t;

    // Removes a value from the hashset. If the value is not present, it returns 0
    // else 1.
    fn hashset_remove(ptr: *mut c_hashset, value: uint16_t) -> uint32_t;
}

// The HashSet interface exposed to the user only tracks the pointer to the
// low-level c_hashset structure. All methods defined on this structure act as
// wrappers and call into the C implementation.
//
// The implementation is currently not generic over the contained type.
pub struct HashSet<T> {
    ptr: *mut c_hashset,
    _marker: PhantomData<T>,
}

// Wrapper methods which ensure that consumer code does not have to make calls
// to unsafe C-FFI functions.
impl<T> HashSet<T> {
    pub fn new() -> Self {
        unsafe { HashSet { ptr: hashset_new(), _marker: Default::default() } }
    }

    pub fn insert(&mut self, value: uint16_t) -> bool {
        unsafe { hashset_insert(self.ptr, value) != 0 }
    }

    pub fn contains(&self, value: &uint16_t) -> bool {
        unsafe { hashset_contains(self.ptr, *value) != 0 }
    }

    pub fn remove(&mut self, value: uint16_t) -> bool {
        unsafe { hashset_remove(self.ptr, value) != 0 }
    }
}
