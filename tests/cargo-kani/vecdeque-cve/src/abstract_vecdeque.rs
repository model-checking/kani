// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! An *abstract* implementation of `VecDeque` useful for verification results.
//!
//! We use the ideas of use *parametricity* and *abstraction* to soundly
//! over-approximate the real implementation. This means that we "throwaway"
//! the underlying storage buffer and only model its `capacity`.
//!
//! This is useful from a verification perspective because we can write proof
//! harnesses that show that the (abstracted) methods of our queue maintain a
//! resource invariant. This means that we have confidence that the fix to the
//! CVE covers all possible cases.

use kani::Arbitrary;

///
/// Based on src/alloc/collections/vec_deque/mod.rs
///
/// Changes from the implementation are marked `Kani change`.
/// Generic type `T` is implicit (but we assume it is not a ZST)
/// We don't model alloc type `A`
struct AbstractVecDeque {
    tail: usize,
    head: usize,
    buf: AbstractRawVec,
}

impl kani::Arbitrary for AbstractVecDeque {
    fn any() -> Self {
        let value = AbstractVecDeque { tail: kani::any(), head: kani::any(), buf: kani::any() };
        kani::assume(value.is_valid());
        value
    }
}

impl AbstractVecDeque {
    fn is_valid(&self) -> bool {
        self.tail < self.cap() && self.head < self.cap() && is_nonzero_pow2(self.cap())
    }

    // what we call the *buf capacity*
    fn cap(&self) -> usize {
        self.buf.capacity()
    }

    // what we call the *usable capacity*
    fn capacity(&self) -> usize {
        self.cap() - 1
    }

    pub fn len(&self) -> usize {
        count(self.tail, self.head, self.cap())
    }

    // version of `reserve` with cve fixed
    pub fn reserve(&mut self, additional: usize) {
        let old_cap = self.cap();
        let used_cap = self.len() + 1;
        let new_cap = used_cap
            .checked_add(additional)
            .and_then(|needed_cap| needed_cap.checked_next_power_of_two())
            .expect("capacity overflow");

        if new_cap > old_cap {
            self.buf.reserve_exact(used_cap, new_cap - used_cap);
            unsafe {
                self.handle_capacity_increase(old_cap);
            }
        }
    }

    // version of `reserve` with cve
    pub fn reserve_with_cve(&mut self, additional: usize) {
        let old_cap = self.cap();
        let used_cap = self.len() + 1;
        let new_cap = used_cap
            .checked_add(additional)
            .and_then(|needed_cap| needed_cap.checked_next_power_of_two())
            .expect("capacity overflow");

        if new_cap > self.capacity() {
            self.buf.reserve_exact(used_cap, new_cap - used_cap);
            unsafe {
                self.handle_capacity_increase(old_cap);
            }
        }
    }

    unsafe fn handle_capacity_increase(&mut self, old_capacity: usize) {
        let new_capacity = self.cap();

        if self.tail <= self.head {
            // A
            // Nop
        } else if self.head < old_capacity - self.tail {
            // B
            unsafe {
                self.copy_nonoverlapping(old_capacity, 0, self.head);
            }
            self.head += old_capacity;
            debug_assert!(self.head > self.tail);
        } else {
            // C
            let new_tail = new_capacity - (old_capacity - self.tail);
            unsafe {
                self.copy_nonoverlapping(new_tail, self.tail, old_capacity - self.tail);
            }
            self.tail = new_tail;
            debug_assert!(self.head < self.tail);
        }
        debug_assert!(self.head < self.cap());
        debug_assert!(self.tail < self.cap());
        debug_assert!(self.cap().count_ones() == 1);
    }

    // Kani change: abstract by not doing any copies
    unsafe fn copy_nonoverlapping(&self, dst: usize, src: usize, len: usize) {
        assert!(dst + len <= self.cap());
        assert!(src + len <= self.cap());
    }

    pub fn remove(&mut self, index: usize) -> Option<()> {
        if self.is_empty() || self.len() <= index {
            return None;
        }

        let idx = self.wrap_add(self.tail, index);

        // Kani change: return () instead of a T value
        let elem = Some(());
        // unsafe { Some(self.buffer_read(idx)) };

        let distance_to_tail = index;
        let distance_to_head = self.len() - index;

        let contiguous = self.is_contiguous();

        match (contiguous, distance_to_tail <= distance_to_head, idx >= self.tail) {
            (true, true, _) => unsafe {
                self.copy(self.tail + 1, self.tail, index);
                self.tail += 1;
            },
            (true, false, _) => unsafe {
                self.copy(idx, idx + 1, self.head - idx - 1);
                self.head -= 1;
            },
            (false, true, true) => unsafe {
                self.copy(self.tail + 1, self.tail, index);
                self.tail = self.wrap_add(self.tail, 1);
            },
            (false, false, false) => unsafe {
                self.copy(idx, idx + 1, self.head - idx - 1);
                self.head -= 1;
            },
            (false, false, true) => {
                unsafe {
                    // draw in elements in the tail section
                    self.copy(idx, idx + 1, self.cap() - idx - 1);

                    // Prevents underflow.
                    if self.head != 0 {
                        // copy first element into empty spot
                        self.copy(self.cap() - 1, 0, 1);

                        // move elements in the head section backwards
                        self.copy(0, 1, self.head - 1);
                    }

                    self.head = self.wrap_sub(self.head, 1);
                }
            }
            (false, true, false) => {
                unsafe {
                    // draw in elements up to idx
                    self.copy(1, 0, idx);

                    // copy last element into empty spot
                    self.copy(0, self.cap() - 1, 1);

                    // move elements from tail to end forward, excluding the last one
                    self.copy(self.tail + 1, self.tail, self.cap() - self.tail - 1);

                    self.tail = self.wrap_add(self.tail, 1);
                }
            }
        }

        elem
    }

    // Kani change: abstract by not doing any copies
    unsafe fn copy(&self, dst: usize, src: usize, len: usize) {
        assert!(dst + len <= self.cap());
        assert!(src + len <= self.cap());
    }

    fn wrap_index(&self, idx: usize) -> usize {
        wrap_index(idx, self.cap())
    }

    fn wrap_add(&self, idx: usize, addend: usize) -> usize {
        wrap_index(idx.wrapping_add(addend), self.cap())
    }

    fn wrap_sub(&self, idx: usize, subtrahend: usize) -> usize {
        wrap_index(idx.wrapping_sub(subtrahend), self.cap())
    }

    pub fn is_empty(&self) -> bool {
        self.tail == self.head
    }

    fn is_contiguous(&self) -> bool {
        self.tail <= self.head
    }
}

fn wrap_index(index: usize, size: usize) -> usize {
    // size is always a power of 2
    assert!(size.is_power_of_two());
    index & (size - 1)
}

fn count(tail: usize, head: usize, size: usize) -> usize {
    // size is always a power of 2
    assert!(is_nonzero_pow2(size));
    (head.wrapping_sub(tail)) & (size - 1)
}

///
/// Based on src/alloc/raw_vec.rs
///
/// Generic type `T` is implicit (but we assume it is not a ZST)
struct AbstractRawVec {
    /* ptr: Unique<T> removed */
    cap: usize,
    /* alloc: A removed */
}

impl kani::Arbitrary for AbstractRawVec {
    fn any() -> Self {
        AbstractRawVec { cap: kani::any() }
    }
}

impl AbstractRawVec {
    pub fn capacity(&self) -> usize {
        self.cap
    }

    pub fn reserve_exact(&mut self, len: usize, additional: usize) {
        handle_reserve(self.try_reserve_exact(len, additional));
    }

    pub fn try_reserve_exact(
        &mut self,
        len: usize,
        additional: usize,
    ) -> Result<(), TryReserveError> {
        if self.needs_to_grow(len, additional) { self.grow_exact(len, additional) } else { Ok(()) }
    }

    fn needs_to_grow(&self, len: usize, additional: usize) -> bool {
        additional > self.capacity().wrapping_sub(len)
    }

    // Kani change: abstract this function
    fn grow_exact(&mut self, len: usize, additional: usize) -> Result<(), TryReserveError> {
        let cap = len.checked_add(additional).ok_or(TryReserveErrorKind::CapacityOverflow)?;
        self.cap = cap; //< models `set_ptr`
        Ok(())
    }
}

fn handle_reserve(result: Result<(), TryReserveError>) {
    match result.map_err(|e| e.kind()) {
        Err(TryReserveErrorKind::CapacityOverflow) => capacity_overflow(),
        Err(TryReserveErrorKind::AllocError) => handle_alloc_error(),
        Ok(()) => { /* yay */ }
    }
}

fn capacity_overflow() {
    assert!(false);
}

fn handle_alloc_error() {
    assert!(false);
}

///
/// Based on src/alloc/collections/mod.rs
///
pub struct TryReserveError {
    kind: TryReserveErrorKind,
}

impl TryReserveError {
    pub fn kind(&self) -> TryReserveErrorKind {
        self.kind.clone()
    }
}

#[derive(Copy, Clone)]
pub enum TryReserveErrorKind {
    CapacityOverflow,
    AllocError,
}

impl From<TryReserveErrorKind> for TryReserveError {
    fn from(kind: TryReserveErrorKind) -> Self {
        Self { kind }
    }
}

// Helpers
fn is_nonzero_pow2(x: usize) -> bool {
    x.count_ones() == 1
}

pub mod verification {
    use super::*;

    // Proof failure expected
    #[kani::proof]
    pub fn abstract_reserve_maintains_invariant_with_cve() {
        let mut q: AbstractVecDeque = kani::any();
        assert!(q.is_valid());
        let used_cap = q.len() + 1;
        let additional: usize = kani::any();
        kani::assume(no_capacity_overflow(used_cap, additional));
        q.reserve_with_cve(additional);
        assert!(q.is_valid());
    }

    // Proof pass expected
    #[kani::proof]
    pub fn abstract_reserve_maintains_invariant_with_cve_fixed() {
        let mut q: AbstractVecDeque = kani::any();
        assert!(q.is_valid());
        let used_cap = q.len() + 1;
        let additional: usize = kani::any();
        kani::assume(no_capacity_overflow(used_cap, additional));
        q.reserve(additional);
        assert!(q.is_valid());
    }

    // Proof pass expected
    #[kani::proof]
    pub fn abstract_remove_maintains_invariant() {
        let mut q: AbstractVecDeque = kani::any();
        assert!(q.is_valid());
        let index: usize = kani::any();
        q.remove(index);
        assert!(q.is_valid());
    }

    // Necessary because `reserve` panics if the new capacity overflows `usize`
    fn no_capacity_overflow(used_cap: usize, additional: usize) -> bool {
        used_cap
            .checked_add(additional)
            .and_then(|needed_cap| needed_cap.checked_next_power_of_two())
            .is_some()
    }
}
