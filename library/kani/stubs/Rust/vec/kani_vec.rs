// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

mod utils;
use utils::libc;

use std::cmp;
use std::convert::TryFrom;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use std::mem;
use std::ops::{Deref, DerefMut, FnMut, Index, IndexMut};
use std::ptr::{drop_in_place, read};
use std::slice;

// __CPROVER_max_malloc_size is dependent on the number of offset bits used to
// represent a pointer variable. By default, this is chosen to be 56, in which
// case the max_malloc_size is 2 ** (offset_bits - 1). We could go as far as to
// assign the default capacity to be the max_malloc_size but that would be overkill.
// Instead, we choose a high-enough value 2 ** 10. Another reason to do
// this is that it would be easier for the solver to reason about memory if multiple
// Vectors are initialized by the abstraction consumer.
//
// For larger array sizes such as 2 ** (31 - 1) we encounter "array size too large
// for flattening" error.
const DEFAULT_CAPACITY: usize = 1024;
const CBMC_MAX_MALLOC_SIZE: usize = 18014398509481984;

// We choose a constant which will ensure that we dont allocate small vectors.
// Small vectors will lead to more resizing operations and hence slowdown in
// verification performance. It is possible for the consumer of this abstraction
// allocate small buffers, specifically using with_capacity() functions. But there
// are no guarantees made about the allocation once it is full. Even then, the
// user can then choose to shrink_to_fit() if they want to play around with
// tight bounds on the Vec capacity.
const MIN_NON_ZERO_CAP: usize = 1024;

// KaniVec implements a fine-grained abstraction of the Vector library for Rust.
// It is aimed to provide a lot more functionality than the other two available
// abstractions - NoBackVec and CVec. KaniVec aims to implement close-to-complete
// compatibility with the Rust Standard Library (RSL) implementation.
//
// The goal of KaniVec is to implement basic operations of the Vec such as push(),
// pop(), append(), insert() in a much simpler way than it is done in the RSL. The
// intuition behind this idea is that with a simple trace, it would be much easier
// for verification techniques such as bounded model checking to reason about
// that piece of code. For that reason, we choose to directly drop down to libc
// functions for low-level operations so that they can be directly translated
// to CBMC primitives. That way, if CBMC performs better through some optimizations
// Kani would too.
//
// We first implement KaniRawVec, an auxiliary data structure which holds a pointer
// to allocated memory and the capacity of the allocation. This abstracts away
// all low-level memory resizing operations from the actual Vec data structure.
// It is also used later to implement KaniIter, an iterator for the KaniVec
// data structure.
//
// We then use KaniRawVec as a member of Vec (KaniVec) which is the interface exposed
// to the public. KaniVec aims to main close-to-complete compatibility with the
// RSL Vec implementation.
//
// An important future work direction here is to abstract other relevant
// data-structures such as Strings and HashMaps but implementing optimization
// for slices seems super crucial. Most customer code deals with slices since
// operations such as sort(), split(), get() which traditionally deal with linear
// data structures are implemented on the slice. The advantage of doing it so
// is that other data structures have to implement coercion to a slice and
// can then get all these methods for free. For instance, this is done for Vec,
// String, etc. Currently, we implement coercion to std::slice primitive type allowing
// us to take benefits of that implementation directly but we could get much better
// performance for real world code if we could develop abstractions for that
// as well. Initial intuitions are that it might be harder since those operations
// are typically not verification-friendly.
//
// Please note that this implementation has not been tested against ZSTs - Zero
// Sized Types and might show unsound behavior.

// KaniRawVec consists of a pointer to allocated memory and another variable tracking
// the capacity of the allocation.
struct KaniRawVec<T> {
    ptr: *const T,
    cap: usize,
}

impl<T> KaniRawVec<T> {
    fn new() -> Self {
        let elem_size = mem::size_of::<T>();
        // NOTE: Currently, this abstraction is not tested against code which has
        // zero-sized types.
        assert!(elem_size != 0);
        // We choose to allocate a Vector of DEFAULT_CAPACITY which is chosen
        // to be a very high value. This way, for most tests, the trace will not
        // generate any resizing operations.
        //
        // An important callout to make here is that this however prevents us
        // from finding buffer overflow bugs. As we always allocate large enough
        // memory, there will always be enough space for writing data after the
        // index crosses the length of the array.
        let cap = DEFAULT_CAPACITY;
        let ptr = unsafe { libc::malloc(cap * elem_size) as *mut T };
        KaniRawVec { ptr, cap }
    }

    fn new_with_capacity(cap: usize) -> Self {
        let elem_size = mem::size_of::<T>();
        // In this case, allocate space for capacity elements as requested.
        let ptr = unsafe { libc::malloc(cap * elem_size) as *mut T };
        KaniRawVec { ptr, cap }
    }

    // Checks if the Vector needs to be resized to allocate additional more elements.
    fn needs_to_grow(&self, len: usize, additional: usize) -> bool {
        additional > self.cap - len
    }

    // grow() and grow_exact() are functions which reallocate the memory to a larger
    // allocation if we run out of space. These are typically called from wrappers
    // such as reserve() and reserve_exact() from Vec. The semantics of both of these
    // functions is similar to that implemented in the RSL. It is important to call
    // out what they are.
    //
    // According to the RSL, the reserve() function is defined as:
    //
    // "Reserves capacity for at least additional more elements to be inserted in
    // the given Vec<T>. The collection may reserve more space to avoid frequent
    // reallocations. After calling reserve, capacity will be greater than or
    // equal to self.len() + additional.
    // Does nothing if capacity is already sufficient."
    //
    // The important point to note here is that it is expected to reserve space
    // for "atleast" additional more elements. Because of which, there cannot be
    // any guarantees made about how much the exact capacity would be after the
    // grow()/reserve() operation is performed.
    //
    // For the purpose of this implementation, we follow the specifics implemented
    // in raw_vec.rs -> grow_amortized(). We choose:
    //
    // Reference: https://doc.rust-lang.org/src/alloc/raw_vec.rs.html#421
    //
    // max ( current_capacity * 2 , current_length + additional ).
    // This ensures exponential growth of the allocated memory and also reduces
    // the number of resizing operations required.
    //
    // We also ensure that the new allocation is greater than a certain minimum
    // that we want to deal with for verification.
    fn grow(&mut self, len: usize, additional: usize) {
        let elem_size = mem::size_of::<T>();
        let req_cap = len + additional;
        let grow_cap = self.cap * 2;

        let new_cap = if req_cap > grow_cap { req_cap } else { grow_cap };
        let new_cap = if MIN_NON_ZERO_CAP > new_cap { MIN_NON_ZERO_CAP } else { new_cap };
        // As per the definition of reserve()
        assert!(new_cap * elem_size <= isize::MAX as usize);
        unsafe {
            self.ptr = libc::realloc(self.ptr as *mut libc::c_void, new_cap * elem_size) as *mut T;
        }
        self.cap = new_cap;
    }

    fn reserve(&mut self, len: usize, additional: usize) {
        if self.needs_to_grow(len, additional) {
            self.grow(len, additional);
        }
    }

    // grow_exact() also poses interesting semantics for the case of our abstraction.
    // According to the RSL:
    //
    // "Reserves the minimum capacity for exactly additional more elements to be inserted in the
    // given Vec<T>. After calling reserve_exact, capacity will be greater than or equal to
    // self.len() + additional. Does nothing if the capacity is already sufficient.
    // Note that the allocator may give the collection more space than it requests. Therefore,
    // capacity can not be relied upon to be precisely minimal. Prefer reserve if future insertions
    // are expected."
    //
    // As can be observed, the capacity cannot be relied upon to be precisely minimal.
    // However, we try to model the RSL behavior as much as we can. Please refer to
    // grow_exact() from kani_vec.rs for more details.
    fn grow_exact(&mut self, len: usize, additional: usize) {
        let elem_size = mem::size_of::<T>();
        let req_cap = len + additional;
        // The RSL implementation checks if we are growing beyond usize::MAX
        // for ZSTs and panics. The idea is that if we need to grow for a ZST,
        // that effectively means that something has gone wrong.
        assert!(elem_size != 0);
        unsafe {
            self.ptr = libc::realloc(self.ptr as *mut libc::c_void, req_cap * elem_size) as *mut T;
        }
        self.cap = req_cap;
    }

    fn reserve_exact(&mut self, len: usize, additional: usize) {
        if self.needs_to_grow(len, additional) {
            self.grow_exact(len, additional);
        }
    }

    // Reallocate memory such that the allocation size is equal to the exact
    // requirement of the Vector. We try to model RSL behavior (refer raw_vec.rs
    // shrink()) but according to the RSL:
    //
    // "It will drop down as close as possible to the length but the allocator
    // may still inform the vector that there is space for a few more elements."
    //
    // Even in this case, no guarantees can be made to ensure that the capacity
    // of the allocationa after shrinking would be exactly equal to the length.
    fn shrink_to_fit(&mut self, len: usize) {
        assert!(len <= self.cap);
        let elem_size = mem::size_of::<T>();
        unsafe {
            self.ptr = libc::realloc(self.ptr as *mut libc::c_void, len * elem_size) as *mut T;
        }
        self.cap = len;
    }

    fn capacity(&self) -> usize {
        self.cap
    }
}

// Since we allocate memory manually, the Drop for KaniVec should ensure that we
// free that allocation. We drop to libc::free since we have a pointer to the memory
// that was allocated by libc::malloc / libc::realloc.
impl<T> Drop for KaniRawVec<T> {
    fn drop(&mut self) {
        unsafe {
            libc::free(self.ptr as *mut _);
        }
    }
}

// In theory, there is no need to track the Allocator here. However, the RSL
// implementation of the Vector is generic over the type of the Allocator that
// it takes. Also, many functions are part of impl Blocks which require that the
// Vec be generic over the type of the Allocator that it takes.
//
// We define an empty trait Allocator which shadows std::alloc::Allocator.
//
// We also define an empty KaniAllocator structure here which serves as the default type
// for the Vec data structure. The Vec implemented as part of the Rust Standard
// Library has the Global allocator as its default.
pub trait Allocator {}

#[derive(Clone, Copy)]
pub struct KaniAllocator {}

impl KaniAllocator {
    pub fn new() -> Self {
        KaniAllocator {}
    }
}

// Implement the Allocator trait
impl Allocator for KaniAllocator {}

// This is the primary Vec abstraction that is exposed to the user. It has a
// KaniRawVec which tracks the underlying memory and values stored in the Vec. We
// also track the length and an allocator instance.
pub struct Vec<T, A: Allocator = KaniAllocator> {
    buf: KaniRawVec<T>,
    len: usize,
    allocator: A,
}

// Impl block for helper functions.
impl<T, A: Allocator> Vec<T, A> {
    fn ptr(&self) -> *mut T {
        self.buf.ptr as *mut T
    }

    fn with_capacity_in(capacity: usize, allocator: A) -> Self {
        Vec { buf: KaniRawVec::new_with_capacity(capacity), len: 0, allocator: allocator }
    }
}

impl<T> Vec<T> {
    pub fn new() -> Self {
        Vec { buf: KaniRawVec::new(), len: 0, allocator: KaniAllocator::new() }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self::with_capacity_in(cap, KaniAllocator::new())
    }

    // A lot of invariants here are not checked:
    // * If the pointer was not allocated via a String/Vec, it is highly likely to be
    // incorrect.
    // * T needs to have the same and alignment as what ptr was allocated with.
    // * length needs to be less than or equal to the capacity.
    // * capacity needs to be capacity that the pointer was allocated with.
    pub unsafe fn from_raw_parts(ptr: *mut T, length: usize, capacity: usize) -> Self {
        // Assert that the alignment of T and the allocated pointer are the same.
        assert_eq!(mem::align_of::<T>(), mem::align_of_val(&ptr));
        // Assert that the length is less than or equal to the capacity
        assert!(length <= capacity);
        // We cannot check if the capacity of the memory pointer to by ptr is
        // atleast "capacity", this is to be assumed.
        let mut v = Vec {
            buf: KaniRawVec::new_with_capacity(capacity),
            len: 0,
            allocator: KaniAllocator::new(),
        };
        unsafe {
            let mut curr_idx: isize = 0;
            while curr_idx < length as isize {
                // The push performed here is cheap as we have already allocated
                // enough capacity to hold the data.
                v.push_unsafe(read(ptr.offset(curr_idx)));
                curr_idx += 1;
            }
        }
        v
    }
}

impl<T, A: Allocator> Vec<T, A> {
    pub fn allocator(&self) -> &A {
        &self.allocator
    }

    pub fn push(&mut self, elem: T) {
        // Check if the buffer needs to grow in size, call grow() in that case.
        if self.len == self.capacity() {
            self.buf.grow(self.len, 1);
        }

        unsafe {
            *self.ptr().offset(self.len as isize) = elem;
        }
        self.len += 1;
    }

    pub fn push_unsafe(&mut self, elem: T) {
        unsafe {
            *self.ptr().offset(self.len as isize) = elem;
        }
        self.len += 1;
    }

    // It is important to note that pop() does not trigger any changes in the
    // underlying allocation capacity.
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            unsafe { Some(read(self.ptr().offset(self.len as isize))) }
        }
    }

    pub fn insert(&mut self, index: usize, elem: T) {
        assert!(index <= self.len);

        // Check if the buffer needs to grow in size, call grow() in that case.
        if self.capacity() < (self.len + 1) {
            self.buf.grow(self.len, 1);
        }

        unsafe {
            if index < self.len {
                // Perform a memmove of all data from the index starting at idx
                // to idx+1 to make space for the element to be inserted
                libc::memmove(
                    self.ptr().offset(index as isize + 1) as *mut libc::c_void,
                    self.ptr().offset(index as isize) as *mut libc::c_void,
                    (self.len - index) * mem::size_of::<T>(),
                );
            }
            *self.ptr().offset(index as isize) = elem;
            self.len += 1;
        }
    }

    pub fn remove(&mut self, index: usize) -> T {
        assert!(index < self.len);

        unsafe {
            self.len -= 1;
            let result = read(self.ptr().offset(index as isize));
            if self.len - index > 0 {
                // Perform a memmove of all data from the index starting at idx + 1
                // to idx to occupy space created by the element which was removed.
                libc::memmove(
                    self.ptr().offset(index as isize) as *mut libc::c_void,
                    self.ptr().offset(index as isize + 1) as *mut libc::c_void,
                    (self.len - index) * mem::size_of::<T>(),
                );
            }
            result
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    // Please refer to grow() and grow_exact() for more details()
    pub fn reserve(&mut self, additional: usize) {
        self.buf.reserve(self.len, additional);
    }

    pub fn reserve_exact(&mut self, additional: usize) {
        self.buf.reserve(self.len, additional);
    }

    // The following safety guarantees must be satisfied:
    //
    // * new_len must be less than or equal to capacity().
    // * The elements at old_len..new_len must be initialized.
    pub unsafe fn set_len(&mut self, new_len: usize) {
        assert!(new_len <= self.capacity());

        self.len = new_len;
    }

    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.ptr()
    }

    // This is possible as we implement the Deref coercion for Vec
    pub fn as_slice(&self) -> &[T] {
        self
    }

    // This is possible as we implement the DerefMut coercion for Vec
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self
    }

    pub fn as_ptr(&self) -> *const T {
        self.buf.ptr
    }

    // According to the RSL:
    //
    // "Shortens the vector, keeping the first len elements and dropping the rest.
    // If len is greater than the vector’s current length, this has no effect.
    // Note that this method has no effect on the allocated capacity of the vector."
    pub fn truncate(&mut self, len: usize) {
        unsafe {
            if len > self.len {
                return;
            }

            // Call drop for elements which are truncated
            let remaining_len = self.len - len;
            while self.len != len {
                self.len -= 1;
                drop_in_place(self.as_mut_ptr().offset(self.len as isize));
            }
        }
    }

    // Clears the vector, removing all values.
    // This method has no effect on the allocated capacity of the vector
    pub fn clear(&mut self) {
        self.truncate(0);
    }

    // Removes an element from the Vector and returns it. The removed element is
    // replaced by the last element of the Vector. This does not preserve ordering,
    // but is O(1) - because we dont perform memory resizing operations.
    pub fn swap_remove(&mut self, index: usize) -> T {
        let len = self.len;
        assert!(index < len);

        unsafe {
            let last = read(self.as_ptr().add(len - 1));
            let hole = self.as_mut_ptr().add(index);
            self.set_len(len - 1);
            let prev_hole = read(hole);
            *hole = last;
            prev_hole
        }
    }

    // According to the RSL:
    // "Returns the number of elements the vector can hold without reallocating."
    // The API consumer cannot rely on the precision of this function.
    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    // Splits the collection into two at the given index.
    //
    // Returns a newly allocated vector containing the elements in the range [at, len). After the
    // call, the original vector will be left containing the elements [0, at) with its previous
    // capacity unchanged.
    pub fn split_off(&mut self, at: usize) -> Self
    where
        A: Clone,
    {
        assert!(at <= self.len);

        let other_len = self.len - at;
        let mut other = Vec::with_capacity_in(other_len, self.allocator().clone());

        unsafe {
            // Copy all the elements from "at" till the end of the Vector through
            // a memcpy which is much cheaper than remove() and push()
            libc::memcpy(
                other.as_mut_ptr() as *mut libc::c_void,
                self.as_ptr().offset(at as isize) as *mut libc::c_void,
                other_len * mem::size_of::<T>(),
            );

            // Set length to point to end of array.
            self.set_len(at);
            other.set_len(other_len);
        }

        other
    }

    pub fn append(&mut self, other: &mut Vec<T, A>) {
        // Reserve enough space to reduce the number of resizing operations
        self.reserve(other.len());
        unsafe {
            libc::memmove(
                self.as_ptr().offset(self.len as isize) as *mut libc::c_void,
                other.as_ptr() as *mut libc::c_void,
                other.len() * mem::size_of::<T>(),
            );
            self.len += other.len();
            other.set_len(0);
        }
    }

    // Resizes the Vec in-place so that len is equal to new_len.
    //
    // If new_len is greater than len, the Vec is extended by the difference, with each additional
    // slot filled with the result of calling the closure f. The return values from f will end up
    // in the Vec in the order they have been generated.
    //
    // If new_len is less than len, the Vec is simply truncated.
    pub fn resize_with<F>(&mut self, new_len: usize, f: F)
    where
        F: FnMut() -> T,
    {
        let len = self.len;

        if new_len > len {
            let additional = new_len - len;
            self.reserve(additional);
            let mut closure = f;
            for _ in 0..additional {
                // This push is cheap as we have already reserved enough space.
                self.push_unsafe(closure());
            }
        } else {
            self.truncate(new_len);
        }
    }

    // The semantics of shrink() and shrink_to_fit() are similar to that of reserve().
    // According to the RSL:
    //
    // "Shrinks the capacity of the vector as much as possible.
    // It will drop down as close as possible to the length but the allocator may still inform the
    // vector that there is space for a few more elements."
    //
    // There cannot be any guarantees made that the capacity will be changed
    // to fit the length of the Vector exactly.
    pub fn shrink_to_fit(&mut self) {
        if self.capacity() > self.len {
            self.buf.shrink_to_fit(self.len);
        }
    }

    // This is an experimental API. According to the RSL:
    //
    // "Shrinks the capacity of the vector with a lower bound.
    // The capacity will remain at least as large as both the length and the supplied value.
    // If the current capacity is less than the lower limit, this is a no-op."
    pub fn shrink_to(&mut self, min_capacity: usize) {
        if self.capacity() > min_capacity {
            let max = if self.len > min_capacity { self.len } else { min_capacity };
            self.buf.shrink_to_fit(max);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn new_in(alloc: A) -> Self {
        Vec { buf: KaniRawVec::new(), len: 0, allocator: alloc }
    }
}

impl<T: Clone, A: Allocator> Vec<T, A> {
    // Resizes the Vec in-place so that len is equal to new_len.
    //
    // If new_len is greater than len, the Vec is extended by the difference, with each additional
    // slot filled with value. If new_len is less than len, the Vec is simply truncated.
    //
    // This method requires T to implement Clone, in order to be able to clone the passed value.
    pub fn resize(&mut self, new_len: usize, value: T) {
        let len = self.len;

        if new_len > len {
            let additional = new_len - len;
            self.reserve(additional);
            for _ in 0..additional {
                // This push is cheap as we have already reserved enough space.
                self.push_unsafe(value.clone());
            }
        } else {
            self.truncate(new_len);
        }
    }

    // Clones and appends all elements in a slice to the Vec.
    //
    // Iterates over the slice other, clones each element, and then appends it to this Vec. The
    // other vector is traversed in-order.
    pub fn extend_from_slice(&mut self, other: &[T]) {
        let other_len = other.len();
        self.reserve(other_len);
        for i in 0..other_len {
            // This push is cheap as we have already reserved enough space.
            self.push_unsafe(other[i].clone());
        }
    }
}

// Drop is codegen for most types, no need to perform any action here.
impl<T, A: Allocator> Drop for Vec<T, A> {
    fn drop(&mut self) {}
}

// Trait implementations for Vec
// We try to implement all major traits for Vec which might be priority for
// our customers.
impl<T> Default for Vec<T> {
    fn default() -> Self {
        Vec::new()
    }
}

impl<T: PartialEq, A: Allocator> PartialEq for Vec<T, A> {
    fn eq(&self, other: &Self) -> bool {
        if self.len != other.len() {
            return false;
        }

        for idx in 0..self.len {
            if self[idx] != other[idx] {
                return false;
            }
        }

        return true;
    }
}

// We implement the PartialEq trait for Vec with other slices by using a generic
// macro. As we implement the Deref coercion, we can perform self[index] and compare
// it with the RHS.
macro_rules! __impl_slice_eq1 {
    ([$($vars:tt)*] $lhs:ty, $rhs:ty) => {
        impl<T, U, $($vars)*> PartialEq<$rhs> for $lhs
        where
            T: PartialEq<U>, A: Allocator
        {
            #[inline]
            fn eq(&self, other: &$rhs) -> bool { self[..] == other[..] }
            #[inline]
            fn ne(&self, other: &$rhs) -> bool { self[..] != other[..] }
        }
    }
}

__impl_slice_eq1! { [A] Vec<T, A>, &[U] }
__impl_slice_eq1! { [A] Vec<T, A>, &mut [U] }
__impl_slice_eq1! { [A] &[T], Vec<U, A> }
__impl_slice_eq1! { [A] &mut [T], Vec<U, A> }
__impl_slice_eq1! { [A, const N: usize] Vec<T, A>, [U; N] }
__impl_slice_eq1! { [A, const N: usize] Vec<T, A>, &[U; N] }

// Coercion support into Deref allows us to benefit from operations on slice
// implemented in the standard library. Quoting the RSL:
//
// "Deref coercion is a convenience that Rust performs on arguments to functions
// and methods. Deref coercion works only on types that implement the Deref trait.
// Deref coercion converts such a type into a reference to another type. Deref coercion
// happens automatically when we pass a reference to a particular type’s value
// as an argument to a function or method that doesn’t match the parameter type
// in the function or method definition. A sequence of calls to the deref method
// converts the type we provided into the type the parameter needs."
//
// For our case, the deref coercion implemented here can convert a Vec into a
// primitive slice type. This allows us to benefit from methods implemented
// on the slice type such as sort(), split(), etc.
impl<T, A: Allocator> Deref for Vec<T, A> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        unsafe { ::std::slice::from_raw_parts(self.ptr(), self.len) }
    }
}

impl<T, A: Allocator> DerefMut for Vec<T, A> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { ::std::slice::from_raw_parts_mut(self.ptr() as *mut T, self.len) }
    }
}

// Clone
impl<T: Clone, A: Allocator + Clone> Clone for Vec<T, A> {
    fn clone(&self) -> Self {
        let mut v = Self::with_capacity_in(self.len, self.allocator.clone());
        for idx in 0..self.len {
            v.push_unsafe(self[idx].clone());
        }
        v
    }

    fn clone_from(&mut self, other: &Self) {
        *self = other.clone();
    }
}

// Hash
impl<T: Hash, A: Allocator> Hash for Vec<T, A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&**self, state)
    }
}

// Index
impl<T, I: ::std::slice::SliceIndex<[T]>, A: Allocator> Index<I> for Vec<T, A> {
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        Index::index(&**self, index)
    }
}

// IndexMut
impl<T, I: ::std::slice::SliceIndex<[T]>, A: Allocator> IndexMut<I> for Vec<T, A> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        IndexMut::index_mut(&mut **self, index)
    }
}

// From the RSL:
//
// "Extend a collection with the contents of an iterator.
// Iterators produce a series of values, and collections can also be thought of
// as a series of values. The Extend trait bridges this gap, allowing you to
// extend a collection by including the contents of that iterator. When extending
// a collection with an already existing key, that entry is updated or, in the
// case of collections that permit multiple entries with equal keys, that
// entry is inserted."
//
// We cannot reserve space for the elements which are added as we dont know
// the size of the iterator. In this case, we perform sequential push operations.
// However because our underlying Vector grows exponential in size, we can be
// sure that we won't perform too many resizing operations.
impl<T, A: Allocator> Extend<T> for Vec<T, A> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for elem in iter.into_iter() {
            self.push(elem);
        }
    }
}

impl<'a, T: Copy + 'a, A: Allocator + 'a> Extend<&'a T> for Vec<T, A> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        for elem in iter.into_iter() {
            self.push(*elem);
        }
    }
}

impl<T: PartialOrd, A: Allocator> PartialOrd for Vec<T, A> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
}

impl<T: Eq, A: Allocator> Eq for Vec<T, A> {}

impl<T: Ord, A: Allocator> Ord for Vec<T, A> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        Ord::cmp(&**self, &**other)
    }
}

impl<T, A: Allocator> AsRef<Vec<T, A>> for Vec<T, A> {
    fn as_ref(&self) -> &Vec<T, A> {
        self
    }
}

impl<T, A: Allocator> AsMut<Vec<T, A>> for Vec<T, A> {
    fn as_mut(&mut self) -> &mut Vec<T, A> {
        self
    }
}

// AsRef to a slice is possible because we implement the Deref coercion.
impl<T, A: Allocator> AsRef<[T]> for Vec<T, A> {
    fn as_ref(&self) -> &[T] {
        self
    }
}

// AsMut to a slice is possible because we implement the Deref coercion
impl<T, A: Allocator> AsMut<[T]> for Vec<T, A> {
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

// Debug
impl<T: fmt::Debug, A: Allocator> fmt::Debug for Vec<T, A> {
    // fmt implementation left empty since we dont care about debug messages
    // and such in the verification case
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

// Create a new Vec from a slice reference
impl<T: Clone> From<&[T]> for Vec<T> {
    fn from(s: &[T]) -> Vec<T> {
        let s_len = s.len();
        // Reserve space for atleast s.len() elements to avoid resizing
        let mut v = Vec::with_capacity(s_len);
        for i in 0..s_len {
            // This push is cheap as we reserve enough space earlier.
            v.push_unsafe(s[i].clone());
        }
        v
    }
}

// Create a new Vec from a slice mut reference
impl<T: Clone> From<&mut [T]> for Vec<T> {
    fn from(s: &mut [T]) -> Vec<T> {
        let s_len = s.len();
        // Reserve space for atleast s.len() elements to avoid resizing
        let mut v = Vec::with_capacity(s_len);
        for i in 0..s_len {
            // This push is cheap as we reserve enough space earlier.
            v.push_unsafe(s[i].clone());
        }
        v
    }
}

// Create a new Vec from an array
impl<T, const N: usize> From<[T; N]> for Vec<T> {
    fn from(s: [T; N]) -> Vec<T> {
        // Reserve space for atleast s.len() elements to avoid resizing
        let mut v = Vec::with_capacity(s.len());
        for elem in s {
            // This push is cheap as we reserve enough space earlier.
            v.push_unsafe(elem);
        }
        v
    }
}

impl From<&str> for Vec<u8> {
    fn from(s: &str) -> Vec<u8> {
        From::from(s.as_bytes())
    }
}

// Gets the entire contents of the `Vec<T>` as an array,
// if its size exactly matches that of the requested array.
impl<T, A: Allocator, const N: usize> TryFrom<Vec<T, A>> for [T; N] {
    type Error = Vec<T, A>;

    fn try_from(mut vec: Vec<T, A>) -> Result<[T; N], Vec<T, A>> {
        if vec.len() != N {
            return Err(vec);
        }

        unsafe {
            vec.set_len(0);
        }

        let array = unsafe { read(vec.as_ptr() as *const [T; N]) };
        Ok(array)
    }
}

// We implement an IntoIterator for (Kani)Vec using a custom structure -
// KaniIter. For KaniIter, we implement KaniRawValIter as a member which stores
// raw pointers to the start and end of memory of the sequence.
struct KaniRawValIter<T> {
    start: *const T,
    end: *const T,
}

impl<T> KaniRawValIter<T> {
    unsafe fn new(slice: &[T]) -> Self {
        KaniRawValIter {
            // The pointer to the slice marks its beginning
            start: slice.as_ptr(),
            end: if mem::size_of::<T>() == 0 {
                // Handle ZST (Zero-sized types)
                ((slice.as_ptr() as usize) + slice.len()) as *const _
            } else if slice.len() == 0 {
                // If the length of the slice is 0, the pointer to the slice also
                // marks its end
                slice.as_ptr()
            } else {
                // For the general case, compute offset from the start by counting
                // slice.len() elements.
                slice.as_ptr().offset(slice.len() as isize)
            },
        }
    }
}

// An interface for dealing with iterators.
impl<T> Iterator for KaniRawValIter<T> {
    type Item = T;

    // Yield the next element of the sequence. This method changes the internal
    // state of the iterator.
    fn next(&mut self) -> Option<T> {
        // If we have already reached the end, yield a None value. According to
        // the documentation, individual implementations may or may not choose
        // to return a Some() again at some point. In our case, we dont.
        if self.start == self.end {
            None
        } else {
            unsafe {
                let result = read(self.start);
                self.start = if mem::size_of::<T>() == 0 {
                    // Handle ZSTs correctly
                    (self.start as usize + 1) as *const _
                } else {
                    // For the general case, offset increment the start by 1.
                    self.start.offset(1)
                };
                Some(result)
            }
        }
    }
}

// An iterator able to yield elements from both ends.
//
// Something that implements DoubleEndedIterator has one extra capability over
// something that implements Iterator: the ability to also take Items from the back,
// as well as the front.
//
// once a DoubleEndedIterator returns None from a next_back(), calling it again
// may or may not ever return Some again
impl<T> DoubleEndedIterator for KaniRawValIter<T> {
    fn next_back(&mut self) -> Option<T> {
        // If we have already consumed the iterator, return a None. According to
        // the documentation, individual implementations may or may not choose
        // to return a Some() again at some point. In our case, we dont.
        if self.start == self.end {
            None
        } else {
            unsafe {
                self.end = if mem::size_of::<T>() == 0 {
                    // Handle ZSTs
                    (self.end as usize - 1) as *const _
                } else {
                    // Offset decrement the end by 1
                    self.end.offset(-1)
                };
                // Read from end and wrap around a Some()
                Some(read(self.end))
            }
        }
    }
}

// KaniIntoIter contains a KaniRawVec and KaniRawValIter to track the Vector and
// the Iterator. This exposes a public interface which can be used with Vec.
pub struct KaniIntoIter<T: Sized> {
    _buf: KaniRawVec<T>,
    iter: KaniRawValIter<T>,
}

impl<T: Sized> Iterator for KaniIntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.iter.next()
    }
}

impl<T: Sized> DoubleEndedIterator for KaniIntoIter<T> {
    fn next_back(&mut self) -> Option<T> {
        self.iter.next_back()
    }
}

// Implement IntoIterator for Vec
//
// By implementing IntoIterator for a type, you define how it will be converted
// to an iterator.
impl<T, A: Allocator> IntoIterator for Vec<T, A> {
    type Item = T;
    type IntoIter = KaniIntoIter<T>;

    fn into_iter(self) -> KaniIntoIter<T> {
        unsafe {
            let iter = KaniRawValIter::new(&self);
            let buf = read(&self.buf);
            // into_iter() takes self by value, and it consumes that collection.
            // For that reason, we need to ensure that the destructor for the Vec
            // is not called since that will free the underlying buffer. In that
            // case, we need to take ownership of the data while making sure
            // that the destructor is not called. mem::forget allows us to do
            // that. We implement a Drop for KaniIntoIter to ensure that elements
            // which were not yielded are dropped appropriately.
            //
            // For reference: https://doc.rust-lang.org/nomicon/vec-into-iter.html
            mem::forget(self);

            KaniIntoIter { iter, _buf: buf }
        }
    }
}

// FromIterator defines how a Vec will be created from an Iterator.
impl<T> FromIterator<T> for Vec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Vec<T> {
        let mut v = Vec::new();
        for elem in iter.into_iter() {
            v.push_unsafe(elem);
        }
        v
    }
}

// IntoIterator defines how we can convert a Vec into a struct which implements
// Iterator. For our case, we choose std::Iter.
impl<'a, T, A: Allocator> IntoIterator for &'a Vec<T, A> {
    type Item = &'a T;
    type IntoIter = slice::Iter<'a, T>;

    fn into_iter(self) -> slice::Iter<'a, T> {
        self.iter()
    }
}

impl<'a, T, A: Allocator> IntoIterator for &'a mut Vec<T, A> {
    type Item = &'a mut T;
    type IntoIter = slice::IterMut<'a, T>;

    fn into_iter(self) -> slice::IterMut<'a, T> {
        self.iter_mut()
    }
}

// Here, we define the kani_vec! macro which behaves similar to the vec! macro
// found in the std prelude. If we try to override the vec! macro, we get error:
//
//     = note: `vec` could refer to a macro from prelude
//     note: `vec` could also refer to the macro defined here
//
// Relevant Zulip stream:
// https://rust-lang.zulipchat.com/#narrow/stream/122651-general/topic/Override.20prelude.20macro
//
// The workaround for now is to define a new macro. kani_vec! will initialize a new
// Vec based on its definition in this file. We support two types of initialization
// expressions:
//
// [ elem; count] -  initialize a Vector with element value `elem` occurring count times.
// [ elem1, elem2, ...] - initialize a Vector with elements elem1, elem2...
#[cfg(abs_type = "kani")]
#[macro_export]
macro_rules! kani_vec {
  ( $val:expr ; $count:expr ) =>
    ({
      // Reserve space for atleast $count elements to avoid resizing operations
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
