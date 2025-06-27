// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(slice_range)]
#![feature(extend_one)]
#![feature(try_reserve_kind)]
#![feature(allocator_api)]
#![feature(dropck_eyepatch)]
#![feature(rustc_attrs)]
#![feature(core_intrinsics)]
#![feature(ptr_internals)]
#![feature(rustc_allow_const_fn_unstable)]
#![allow(internal_features)]

#[cfg(disable_debug_asserts)]
macro_rules! debug_assert {
    ( $( $x:expr ),* ) => {};
}

mod abstract_vecdeque;
mod cve;
mod fixed;
mod raw_vec;
use abstract_vecdeque::*;

const MAX_CAPACITY: usize = usize::MAX >> 2;

/// This module uses a version of VecDeque that includes the CVE fix.
mod fixed_proofs {
    use crate::MAX_CAPACITY;
    use crate::fixed::VecDeque;

    /// Minimal example that we no longer expect to fail
    #[kani::proof]
    pub fn minimal_example_with_cve_fixed() {
        let mut q = VecDeque::with_capacity(7);
        q.push_front(0);
        q.reserve(6);
        q.push_back(0);
    }

    /// Symbolic example that causes Kani timeout
    /// Hidden behind a flag so `cargo kani` won't pick this harness up by
    /// default
    #[cfg(enable_symbolic_example_with_cve_fixed)]
    #[kani::proof]
    pub fn symbolic_example_with_cve_fixed() {
        let usable_capacity = kani::any();
        kani::assume(usable_capacity < MAX_CAPACITY);
        let mut q = VecDeque::with_capacity(usable_capacity);
        q.push_front(0);
        let additional = kani::any();
        q.reserve(additional);
        q.push_back(0);
    }

    /// Verify that a request to reserve space that is already available is a no-op.
    #[kani::proof]
    pub fn reserve_available_capacity_is_no_op() {
        // Start with a default VecDeque object (default capacity: 7).
        let mut vec_deque = VecDeque::<u8>::new();
        let old_capacity = vec_deque.capacity();

        // Insert an element to empty VecDeque.
        vec_deque.push_front(kani::any());

        // Reserve space to *any* value that is less than or equal to available capacity.
        let new_capacity: usize = kani::any();
        let available = old_capacity - vec_deque.len();
        kani::assume(new_capacity <= available);
        vec_deque.reserve(new_capacity);

        // Verify that capacity should stay the same.
        assert_eq!(vec_deque.capacity(), old_capacity);
    }

    /// Verify that a request to reserve space that is not available triggers a buffer resize.
    #[kani::proof]
    pub fn reserve_more_capacity_works() {
        // Start with a default VecDeque object (default capacity: 7).
        let mut vec_deque = VecDeque::<u8>::new();
        let old_capacity = vec_deque.capacity();

        // Insert an element to empty VecDeque.
        vec_deque.push_front(kani::any());

        // Reserve space to *any* value that is more than available capacity.
        let new_capacity: usize = kani::any();
        let available = old_capacity - vec_deque.len();
        kani::assume(new_capacity > available);
        kani::assume(new_capacity <= (MAX_CAPACITY - vec_deque.len()));
        vec_deque.reserve(new_capacity);

        // Verify that capacity should stay the same.
        assert!(vec_deque.capacity() > old_capacity);
    }
}

mod cve_proofs {
    // Modified version of vec_deque with reserve issue.
    use crate::MAX_CAPACITY;
    use crate::cve::VecDeque;

    /// Minimal example that we expect to fail
    #[kani::proof]
    pub fn minimal_example_with_cve_should_fail() {
        let mut q = VecDeque::with_capacity(7);
        q.push_front(0);
        q.reserve(6);
        q.push_back(0);
    }

    /// Verify that a request to reserve space that is already available is a no-op.
    /// We expect this to fail
    #[kani::proof]
    pub fn reserve_available_capacity_should_fail() {
        // Start with a default VecDeque object (default capacity: 7).
        let mut vec_deque = VecDeque::<u8>::new();
        let old_capacity = vec_deque.capacity();

        // Insert an element to empty VecDeque.
        vec_deque.push_front(kani::any());

        // Reserve space to *any* value that is less than or equal to available capacity.
        let new_capacity: usize = kani::any();
        let available = old_capacity - vec_deque.len();
        kani::assume(new_capacity <= available);
        vec_deque.reserve(new_capacity);

        // Verify that capacity should stay the same.
        assert_eq!(vec_deque.capacity(), old_capacity);
    }

    /// Verify that a request to reserve space that is not available triggers a buffer resize.
    #[kani::proof]
    pub fn reserve_more_capacity_still_works() {
        // Start with a default VecDeque object (default capacity: 7).
        let mut vec_deque = VecDeque::<u8>::new();
        let old_capacity = vec_deque.capacity();

        // Insert an element to empty VecDeque.
        vec_deque.push_front(kani::any());

        // Reserve space to *any* value that is more than available capacity.
        let new_capacity: usize = kani::any();
        let available = old_capacity - vec_deque.len();
        kani::assume(new_capacity > available);
        kani::assume(new_capacity <= (MAX_CAPACITY - vec_deque.len()));
        vec_deque.reserve(new_capacity);

        // Verify that capacity should stay the same.
        assert!(vec_deque.capacity() > old_capacity);
    }
}
