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

mod cve;
mod fixed;
mod raw_vec;

const MAX_CAPACITY: usize = usize::MAX >> 1;

/// This module uses a version of VecDeque that includes the CVE fix.
mod fixed_proofs {
    use crate::fixed::VecDeque;
    use crate::MAX_CAPACITY;

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
    use crate::cve::VecDeque;
    use crate::MAX_CAPACITY;

    /// Verify that a request to reserve space that is already available is a no-op.
    /// This harness uses a version of VecDeque that includes the CVE fix.
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
