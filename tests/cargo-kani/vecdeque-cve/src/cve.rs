// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This is a modified version of std collections/vec_deque/mod.rs

//! A double-ended queue (deque) implemented with a growable ring buffer.
//!
//! This queue has *O*(1) amortized inserts and removals from both ends of the
//! container. It also has *O*(1) indexing like a vector. The contained elements
//! are not required to be copyable, and the queue will be sendable if the
//! contained type is sendable.

#[allow(unsafe_op_in_unsafe_fn)]
use core::ops::{Range, RangeBounds};
use core::slice;

use crate::raw_vec::RawVec;
use std::alloc::{Allocator, Global};
pub use std::collections::vec_deque::*;
use std::{cmp, mem, ptr};

const INITIAL_CAPACITY: usize = 7; // 2^3 - 1
const MINIMUM_CAPACITY: usize = 1; // 2 - 1

const MAXIMUM_ZST_CAPACITY: usize = 1 << (usize::BITS - 1); // Largest possible power of two

/// A double-ended queue implemented with a growable ring buffer.
///
/// The "default" usage of this type as a queue is to use [`push_back`] to add to
/// the queue, and [`pop_front`] to remove from the queue. [`extend`] and [`append`]
/// push onto the back in this manner, and iterating over `VecDeque` goes front
/// to back.
///
/// A `VecDeque` with a known list of items can be initialized from an array:
///
/// ```
/// use std::collections::VecDeque;
///
/// let deq = VecDeque::from([-1, 0, 1]);
/// ```
///
/// Since `VecDeque` is a ring buffer, its elements are not necessarily contiguous
/// in memory. If you want to access the elements as a single slice, such as for
/// efficient sorting, you can use [`make_contiguous`]. It rotates the `VecDeque`
/// so that its elements do not wrap, and returns a mutable slice to the
/// now-contiguous element sequence.
///
/// [`push_back`]: VecDeque::push_back
/// [`pop_front`]: VecDeque::pop_front
/// [`extend`]: VecDeque::extend
/// [`append`]: VecDeque::append
/// [`make_contiguous`]: VecDeque::make_contiguous
//#[cfg_attr(not(test), rustc_diagnostic_item = "VecDeque")]
//#[stable(feature = "rust1", since = "1.0.0")]
#[rustc_insignificant_dtor]
pub struct VecDeque<
    T,
    //#[unstable(feature = "allocator_api", issue = "32838")]
    A: Allocator = Global,
> {
    // tail and head are pointers into the buffer. Tail always points
    // to the first element that could be read, Head always points
    // to where data should be written.
    // If tail == head the buffer is empty. The length of the ringbuffer
    // is defined as the distance between the two.
    tail: usize,
    head: usize,
    buf: RawVec<T, A>,
}

//#[stable(feature = "rust1", since = "1.0.0")]
#[cfg(real_std)]
impl<T: Clone, A: Allocator + Clone> Clone for VecDeque<T, A> {
    fn clone(&self) -> Self {
        let mut deq = Self::with_capacity_in(self.len(), self.allocator().clone());
        deq.extend(self.iter().cloned());
        deq
    }

    fn clone_from(&mut self, other: &Self) {
        self.truncate(other.len());

        let mut iter = PairSlices::from(self, other);
        while let Some((dst, src)) = iter.next() {
            dst.clone_from_slice(&src);
        }

        if iter.has_remainder() {
            for remainder in iter.remainder() {
                self.extend(remainder.iter().cloned());
            }
        }
    }
}

//#[stable(feature = "rust1", since = "1.0.0")]
unsafe impl<#[may_dangle] T, A: Allocator> Drop for VecDeque<T, A> {
    fn drop(&mut self) {
        /// Runs the destructor for all items in the slice when it gets dropped (normally or
        /// during unwinding).
        struct Dropper<'a, T>(&'a mut [T]);

        impl<'a, T> Drop for Dropper<'a, T> {
            fn drop(&mut self) {
                unsafe {
                    ptr::drop_in_place(self.0);
                }
            }
        }

        //--- Modified drop.
        let _head = self.head;
        let _tail = self.tail;
        let _buf = unsafe { self.buffer_as_mut_slice() };
        unimplemented!("Need to finish this.");
        /*
        unsafe {
            let _back_dropper = Dropper(back);
            // use drop for [T]
            ptr::drop_in_place(front);
        }
         */
        // RawVec handles deallocation
    }
}

//#[stable(feature = "rust1", since = "1.0.0")]
impl<T> Default for VecDeque<T> {
    /// Creates an empty deque.
    #[inline]
    fn default() -> VecDeque<T> {
        VecDeque::new()
    }
}

impl<T, A: Allocator> VecDeque<T, A> {
    /// Marginally more convenient
    #[inline]
    fn ptr(&self) -> *mut T {
        self.buf.ptr()
    }

    /// Marginally more convenient
    #[inline]
    fn cap(&self) -> usize {
        if mem::size_of::<T>() == 0 {
            // For zero sized types, we are always at maximum capacity
            MAXIMUM_ZST_CAPACITY
        } else {
            self.buf.capacity()
        }
    }

    /// Turn ptr into a mut slice
    #[inline]
    unsafe fn buffer_as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.ptr(), self.cap()) }
    }

    /// Writes an element into the buffer, moving it.
    #[inline]
    unsafe fn buffer_write(&mut self, off: usize, value: T) {
        unsafe {
            ptr::write(self.ptr().add(off), value);
        }
    }

    /// Returns `true` if the buffer is at full capacity.
    #[inline]
    fn is_full(&self) -> bool {
        self.cap() - self.len() == 1
    }

    /// Returns the index in the underlying buffer for a given logical element
    /// index + addend.
    #[inline]
    fn wrap_add(&self, idx: usize, addend: usize) -> usize {
        wrap_index(idx.wrapping_add(addend), self.cap())
    }

    /// Returns the index in the underlying buffer for a given logical element
    /// index - subtrahend.
    #[inline]
    fn wrap_sub(&self, idx: usize, subtrahend: usize) -> usize {
        wrap_index(idx.wrapping_sub(subtrahend), self.cap())
    }

    /// Copies a contiguous block of memory len long from src to dst
    #[inline]
    unsafe fn copy_nonoverlapping(&self, dst: usize, src: usize, len: usize) {
        debug_assert!(
            dst + len <= self.cap(),
            "cno dst={} src={} len={} cap={}",
            dst,
            src,
            len,
            self.cap()
        );
        debug_assert!(
            src + len <= self.cap(),
            "cno dst={} src={} len={} cap={}",
            dst,
            src,
            len,
            self.cap()
        );
        unsafe {
            ptr::copy_nonoverlapping(self.ptr().add(src), self.ptr().add(dst), len);
        }
    }

    /// Frobs the head and tail sections around to handle the fact that we
    /// just reallocated. Unsafe because it trusts old_capacity.
    #[inline]
    unsafe fn handle_capacity_increase(&mut self, old_capacity: usize) {
        let new_capacity = self.cap();

        // Move the shortest contiguous section of the ring buffer
        //    T             H
        //   [o o o o o o o . ]
        //    T             H
        // A [o o o o o o o . . . . . . . . . ]
        //        H T
        //   [o o . o o o o o ]
        //          T             H
        // B [. . . o o o o o o o . . . . . . ]
        //              H T
        //   [o o o o o . o o ]
        //              H                 T
        // C [o o o o o . . . . . . . . . o o ]

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
}

impl<T> VecDeque<T> {
    /// Creates an empty deque.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::VecDeque;
    ///
    /// let deque: VecDeque<u32> = VecDeque::new();
    /// ```
    #[inline]
    //#[stable(feature = "rust1", since = "1.0.0")]
    #[must_use]
    pub fn new() -> VecDeque<T> {
        VecDeque::new_in(Global)
    }

    /// Creates an empty deque with space for at least `capacity` elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::VecDeque;
    ///
    /// let deque: VecDeque<u32> = VecDeque::with_capacity(10);
    /// ```
    #[inline]
    //#[stable(feature = "rust1", since = "1.0.0")]
    #[must_use]
    pub fn with_capacity(capacity: usize) -> VecDeque<T> {
        Self::with_capacity_in(capacity, Global)
    }
}

impl<T, A: Allocator> VecDeque<T, A> {
    /// Creates an empty deque.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::VecDeque;
    ///
    /// let deque: VecDeque<u32> = VecDeque::new();
    /// ```
    #[inline]
    //#[unstable(feature = "allocator_api", issue = "32838")]
    fn new_in(alloc: A) -> VecDeque<T, A> {
        VecDeque::with_capacity_in(INITIAL_CAPACITY, alloc)
    }

    /// Creates an empty deque with space for at least `capacity` elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::VecDeque;
    ///
    /// let deque: VecDeque<u32> = VecDeque::with_capacity(10);
    /// ```
    //#[unstable(feature = "allocator_api", issue = "32838")]
    fn with_capacity_in(capacity: usize, alloc: A) -> VecDeque<T, A> {
        assert!(capacity < 1_usize << usize::BITS - 1, "capacity overflow");
        // +1 since the ringbuffer always leaves one space empty
        let cap = cmp::max(capacity + 1, MINIMUM_CAPACITY + 1).next_power_of_two();

        VecDeque { tail: 0, head: 0, buf: RawVec::with_capacity_in(cap, alloc) }
    }

    /// Provides a reference to the element at the given index.
    ///
    /// Element at index 0 is the front of the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::VecDeque;
    ///
    /// let mut buf = VecDeque::new();
    /// buf.push_back(3);
    /// buf.push_back(4);
    /// buf.push_back(5);
    /// assert_eq!(buf.get(1), Some(&4));
    /// ```
    //#[stable(feature = "rust1", since = "1.0.0")]
    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.len() {
            let idx = self.wrap_add(self.tail, index);
            unsafe { Some(&*self.ptr().add(idx)) }
        } else {
            None
        }
    }

    /// Provides a mutable reference to the element at the given index.
    ///
    /// Element at index 0 is the front of the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::VecDeque;
    ///
    /// let mut buf = VecDeque::new();
    /// buf.push_back(3);
    /// buf.push_back(4);
    /// buf.push_back(5);
    /// if let Some(elem) = buf.get_mut(1) {
    ///     *elem = 7;
    /// }
    ///
    /// assert_eq!(buf[1], 7);
    /// ```
    //#[stable(feature = "rust1", since = "1.0.0")]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index < self.len() {
            let idx = self.wrap_add(self.tail, index);
            unsafe { Some(&mut *self.ptr().add(idx)) }
        } else {
            None
        }
    }

    /// Returns the number of elements the deque can hold without
    /// reallocating.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::VecDeque;
    ///
    /// let buf: VecDeque<i32> = VecDeque::with_capacity(10);
    /// assert!(buf.capacity() >= 10);
    /// ```
    #[inline]
    //#[stable(feature = "rust1", since = "1.0.0")]
    pub fn capacity(&self) -> usize {
        self.cap() - 1
    }

    /// Reserves the minimum capacity for exactly `additional` more elements to be inserted in the
    /// given deque. Does nothing if the capacity is already sufficient.
    ///

    /// Reserves capacity for at least `additional` more elements to be inserted in the given
    /// deque. The collection may reserve more space to avoid frequent reallocations.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity overflows `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::VecDeque;
    ///
    /// let mut buf: VecDeque<i32> = [1].into();
    /// buf.reserve(10);
    /// assert!(buf.capacity() >= 11);
    /// ```
    //#[stable(feature = "rust1", since = "1.0.0")]
    pub fn reserve(&mut self, additional: usize) {
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

    /// Returns the number of elements in the deque.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::VecDeque;
    ///
    /// let mut deque = VecDeque::new();
    /// assert_eq!(deque.len(), 0);
    /// deque.push_back(1);
    /// assert_eq!(deque.len(), 1);
    /// ```
    //#[stable(feature = "rust1", since = "1.0.0")]
    pub fn len(&self) -> usize {
        count(self.tail, self.head, self.cap())
    }

    /// Returns `true` if the deque is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::VecDeque;
    ///
    /// let mut deque = VecDeque::new();
    /// assert!(deque.is_empty());
    /// deque.push_front(1);
    /// assert!(!deque.is_empty());
    /// ```
    //#[stable(feature = "rust1", since = "1.0.0")]
    pub fn is_empty(&self) -> bool {
        self.tail == self.head
    }

    fn range_tail_head<R>(&self, range: R) -> (usize, usize)
    where
        R: RangeBounds<usize>,
    {
        let Range { start, end } = slice::range(range, ..self.len());
        let tail = self.wrap_add(self.tail, start);
        let head = self.wrap_add(self.tail, end);
        (tail, head)
    }

    /// Creates an iterator that covers the specified range in the deque.
    ///
    /// # Panics
    ///
    /// Panics if the starting point is greater than the end point or if
    /// the end point is greater than the length of the deque.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::VecDeque;
    ///
    /// let deque: VecDeque<_> = [1, 2, 3].into();
    /// let range = deque.range(2..).copied().collect::<VecDeque<_>>();
    /// assert_eq!(range, [3]);
    ///
    /// // A full range covers all contents
    /// let all = deque.range(..);
    /// assert_eq!(all.len(), 3);
    /// ```
    #[inline]
    //#[stable(feature = "deque_range", since = "1.51.0")]
    #[cfg(real_std)]
    pub fn range<R>(&self, range: R) -> Iter<'_, T>
    where
        R: RangeBounds<usize>,
    {
        let (tail, head) = self.range_tail_head(range);
        Iter {
            tail,
            head,
            // The shared reference we have in &self is maintained in the '_ of Iter.
            ring: unsafe { self.buffer_as_slice() },
        }
    }

    /// Creates an iterator that covers the specified mutable range in the deque.
    ///
    /// # Panics
    ///
    /// Panics if the starting point is greater than the end point or if
    /// the end point is greater than the length of the deque.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::VecDeque;
    ///
    /// let mut deque: VecDeque<_> = [1, 2, 3].into();
    /// for v in deque.range_mut(2..) {
    ///   *v *= 2;
    /// }
    /// assert_eq!(deque, [1, 2, 6]);
    ///
    /// // A full range covers all contents
    /// for v in deque.range_mut(..) {
    ///   *v *= 2;
    /// }
    /// assert_eq!(deque, [2, 4, 12]);
    /// ```
    #[inline]
    //#[stable(feature = "deque_range", since = "1.51.0")]
    #[cfg(real_std)]
    pub fn range_mut<R>(&mut self, range: R) -> IterMut<'_, T>
    where
        R: RangeBounds<usize>,
    {
        let (tail, head) = self.range_tail_head(range);

        // SAFETY: The internal `IterMut` safety invariant is established because the
        // `ring` we create is a dereferenceable slice for lifetime '_.
        let ring = ptr::slice_from_raw_parts_mut(self.ptr(), self.cap());

        unsafe { IterMut::new(ring, tail, head, PhantomData) }
    }

    /// Removes the specified range from the deque in bulk, returning all
    /// removed elements as an iterator. If the iterator is dropped before
    /// being fully consumed, it drops the remaining removed elements.
    ///
    /// The returned iterator keeps a mutable borrow on the queue to optimize
    /// its implementation.
    ///
    ///
    /// # Panics
    ///
    /// Panics if the starting point is greater than the end point or if
    /// the end point is greater than the length of the deque.
    ///
    /// # Leaking
    ///
    /// If the returned iterator goes out of scope without being dropped (due to
    /// [`mem::forget`], for example), the deque may have lost and leaked
    /// elements arbitrarily, including elements outside the range.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::VecDeque;
    ///
    /// let mut deque: VecDeque<_> = [1, 2, 3].into();
    /// let drained = deque.drain(2..).collect::<VecDeque<_>>();
    /// assert_eq!(drained, [3]);
    /// assert_eq!(deque, [1, 2]);
    ///
    /// // A full range clears all contents, like `clear()` does
    /// deque.drain(..);
    /// assert!(deque.is_empty());
    /// ```
    #[inline]
    //#[stable(feature = "drain", since = "1.6.0")]
    #[cfg(real_std)]
    pub fn drain<R>(&mut self, range: R) -> Drain<'_, T, A>
    where
        R: RangeBounds<usize>,
    {
        // Memory safety
        //
        // When the Drain is first created, the source deque is shortened to
        // make sure no uninitialized or moved-from elements are accessible at
        // all if the Drain's destructor never gets to run.
        //
        // Drain will ptr::read out the values to remove.
        // When finished, the remaining data will be copied back to cover the hole,
        // and the head/tail values will be restored correctly.
        //
        let (drain_tail, drain_head) = self.range_tail_head(range);

        // The deque's elements are parted into three segments:
        // * self.tail  -> drain_tail
        // * drain_tail -> drain_head
        // * drain_head -> self.head
        //
        // T = self.tail; H = self.head; t = drain_tail; h = drain_head
        //
        // We store drain_tail as self.head, and drain_head and self.head as
        // after_tail and after_head respectively on the Drain. This also
        // truncates the effective array such that if the Drain is leaked, we
        // have forgotten about the potentially moved values after the start of
        // the drain.
        //
        //        T   t   h   H
        // [. . . o o x x o o . . .]
        //
        let head = self.head;

        // "forget" about the values after the start of the drain until after
        // the drain is complete and the Drain destructor is run.
        self.head = drain_tail;

        let deque = NonNull::from(&mut *self);
        let iter = Iter {
            tail: drain_tail,
            head: drain_head,
            // Crucially, we only create shared references from `self` here and read from
            // it.  We do not write to `self` nor reborrow to a mutable reference.
            // Hence the raw pointer we created above, for `deque`, remains valid.
            ring: unsafe { self.buffer_as_slice() },
        };

        unsafe { Drain::new(drain_head, head, iter, deque) }
    }

    /// Clears the deque, removing all values.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::VecDeque;
    ///
    /// let mut deque = VecDeque::new();
    /// deque.push_back(1);
    /// deque.clear();
    /// assert!(deque.is_empty());
    /// ```
    //#[stable(feature = "rust1", since = "1.0.0")]
    #[inline]
    #[cfg(real_std)]
    pub fn clear(&mut self) {
        self.truncate(0);
    }

    /// Returns `true` if the deque contains an element equal to the
    /// given value.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::VecDeque;
    ///
    /// let mut deque: VecDeque<u32> = VecDeque::new();
    ///
    /// deque.push_back(0);
    /// deque.push_back(1);
    ///
    /// assert_eq!(deque.contains(&1), true);
    /// assert_eq!(deque.contains(&10), false);
    /// ```
    //#[stable(feature = "vec_deque_contains", since = "1.12.0")]
    #[cfg(real_std)]
    pub fn contains(&self, x: &T) -> bool
    where
        T: PartialEq<T>,
    {
        let (a, b) = self.as_slices();
        a.contains(x) || b.contains(x)
    }

    /// Provides a reference to the front element, or `None` if the deque is
    /// empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::VecDeque;
    ///
    /// let mut d = VecDeque::new();
    /// assert_eq!(d.front(), None);
    ///
    /// d.push_back(1);
    /// d.push_back(2);
    /// assert_eq!(d.front(), Some(&1));
    /// ```
    //#[stable(feature = "rust1", since = "1.0.0")]
    pub fn front(&self) -> Option<&T> {
        self.get(0)
    }

    /// Provides a mutable reference to the front element, or `None` if the
    /// deque is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::VecDeque;
    ///
    /// let mut d = VecDeque::new();
    /// assert_eq!(d.front_mut(), None);
    ///
    /// d.push_back(1);
    /// d.push_back(2);
    /// match d.front_mut() {
    ///     Some(x) => *x = 9,
    ///     None => (),
    /// }
    /// assert_eq!(d.front(), Some(&9));
    /// ```
    //#[stable(feature = "rust1", since = "1.0.0")]
    pub fn front_mut(&mut self) -> Option<&mut T> {
        self.get_mut(0)
    }

    /// Provides a reference to the back element, or `None` if the deque is
    /// empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::VecDeque;
    ///
    /// let mut d = VecDeque::new();
    /// assert_eq!(d.back(), None);
    ///
    /// d.push_back(1);
    /// d.push_back(2);
    /// assert_eq!(d.back(), Some(&2));
    /// ```
    //#[stable(feature = "rust1", since = "1.0.0")]
    pub fn back(&self) -> Option<&T> {
        self.get(self.len().wrapping_sub(1))
    }

    /// Prepends an element to the deque.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::VecDeque;
    ///
    /// let mut d = VecDeque::new();
    /// d.push_front(1);
    /// d.push_front(2);
    /// assert_eq!(d.front(), Some(&2));
    /// ```
    //#[stable(feature = "rust1", since = "1.0.0")]
    pub fn push_front(&mut self, value: T) {
        if self.is_full() {
            self.grow();
        }

        self.tail = self.wrap_sub(self.tail, 1);
        let tail = self.tail;
        unsafe {
            self.buffer_write(tail, value);
        }
    }

    /// Appends an element to the back of the deque.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::VecDeque;
    ///
    /// let mut buf = VecDeque::new();
    /// buf.push_back(1);
    /// buf.push_back(3);
    /// assert_eq!(3, *buf.back().unwrap());
    /// ```
    //#[stable(feature = "rust1", since = "1.0.0")]
    pub fn push_back(&mut self, value: T) {
        if self.is_full() {
            self.grow();
        }

        let head = self.head;
        self.head = self.wrap_add(self.head, 1);
        unsafe { self.buffer_write(head, value) }
    }

    // Double the buffer size. This method is inline(never), so we expect it to only
    // be called in cold paths.
    // This may panic or abort
    #[inline(never)]
    fn grow(&mut self) {
        // Extend or possibly remove this assertion when valid use-cases for growing the
        // buffer without it being full emerge
        debug_assert!(self.is_full());
        let old_cap = self.cap();
        self.buf.reserve_exact(old_cap, old_cap);
        assert!(self.cap() == old_cap * 2);
        unsafe {
            self.handle_capacity_increase(old_cap);
        }
        debug_assert!(!self.is_full());
    }

    /// Modifies the deque in-place so that `len()` is equal to `new_len`,
    /// either by removing excess elements from the back or by appending
    /// elements generated by calling `generator` to the back.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::VecDeque;
    ///
    /// let mut buf = VecDeque::new();
    /// buf.push_back(5);
    /// buf.push_back(10);
    /// buf.push_back(15);
    /// assert_eq!(buf, [5, 10, 15]);
    ///
    /// buf.resize_with(5, Default::default);
    /// assert_eq!(buf, [5, 10, 15, 0, 0]);
    ///
    /// buf.resize_with(2, || unreachable!());
    /// assert_eq!(buf, [5, 10]);
    ///
    /// let mut state = 100;
    /// buf.resize_with(5, || { state += 1; state });
    /// assert_eq!(buf, [5, 10, 101, 102, 103]);
    /// ```
    //#[stable(feature = "vec_resize_with", since = "1.33.0")]
    #[cfg(real_std)]
    pub fn resize_with(&mut self, new_len: usize, generator: impl FnMut() -> T) {
        let len = self.len();

        if new_len > len {
            self.extend(repeat_with(generator).take(new_len - len))
        } else {
            self.truncate(new_len);
        }
    }

    /// Rearranges the internal storage of this deque so it is one contiguous
    /// slice, which is then returned.
    ///
    /// This method does not allocate and does not change the order of the
    /// inserted elements. As it returns a mutable slice, this can be used to
    /// sort a deque.
    ///
    /// Once the internal storage is contiguous, the [`as_slices`] and
    /// [`as_mut_slices`] methods will return the entire contents of the
    /// deque in a single slice.
    ///
    /// [`as_slices`]: VecDeque::as_slices
    /// [`as_mut_slices`]: VecDeque::as_mut_slices
    ///
    /// # Examples
    ///
    /// Sorting the content of a deque.
    ///
    /// ```
    /// use std::collections::VecDeque;
    ///
    /// let mut buf = VecDeque::with_capacity(15);
    ///
    /// buf.push_back(2);
    /// buf.push_back(1);
    /// buf.push_front(3);
    ///
    /// // sorting the deque
    /// buf.make_contiguous().sort();
    /// assert_eq!(buf.as_slices(), (&[1, 2, 3] as &[_], &[] as &[_]));
    ///
    /// // sorting it in reverse order
    /// buf.make_contiguous().sort_by(|a, b| b.cmp(a));
    /// assert_eq!(buf.as_slices(), (&[3, 2, 1] as &[_], &[] as &[_]));
    /// ```
    ///
    /// Getting immutable access to the contiguous slice.
    ///
    /// ```rust
    /// use std::collections::VecDeque;
    ///
    /// let mut buf = VecDeque::new();
    ///
    /// buf.push_back(2);
    /// buf.push_back(1);
    /// buf.push_front(3);
    ///
    /// buf.make_contiguous();
    /// if let (slice, &[]) = buf.as_slices() {
    ///     // we can now be sure that `slice` contains all elements of the deque,
    ///     // while still having immutable access to `buf`.
    ///     assert_eq!(buf.len(), slice.len());
    ///     assert_eq!(slice, &[3, 2, 1] as &[_]);
    /// }
    /// ```
    //#[stable(feature = "deque_make_contiguous", since = "1.48.0")]
    #[cfg(real_std)]
    pub fn make_contiguous(&mut self) -> &mut [T] {
        if self.is_contiguous() {
            let tail = self.tail;
            let head = self.head;
            return unsafe { RingSlices::ring_slices(self.buffer_as_mut_slice(), head, tail).0 };
        }

        let buf = self.buf.ptr();
        let cap = self.cap();
        let len = self.len();

        let free = self.tail - self.head;
        let tail_len = cap - self.tail;

        if free >= tail_len {
            // there is enough free space to copy the tail in one go,
            // this means that we first shift the head backwards, and then
            // copy the tail to the correct position.
            //
            // from: DEFGH....ABC
            // to:   ABCDEFGH....
            unsafe {
                ptr::copy(buf, buf.add(tail_len), self.head);
                // ...DEFGH.ABC
                ptr::copy_nonoverlapping(buf.add(self.tail), buf, tail_len);
                // ABCDEFGH....

                self.tail = 0;
                self.head = len;
            }
        } else if free > self.head {
            // FIXME: We currently do not consider ....ABCDEFGH
            // to be contiguous because `head` would be `0` in this
            // case. While we probably want to change this it
            // isn't trivial as a few places expect `is_contiguous`
            // to mean that we can just slice using `buf[tail..head]`.

            // there is enough free space to copy the head in one go,
            // this means that we first shift the tail forwards, and then
            // copy the head to the correct position.
            //
            // from: FGH....ABCDE
            // to:   ...ABCDEFGH.
            unsafe {
                ptr::copy(buf.add(self.tail), buf.add(self.head), tail_len);
                // FGHABCDE....
                ptr::copy_nonoverlapping(buf, buf.add(self.head + tail_len), self.head);
                // ...ABCDEFGH.

                self.tail = self.head;
                self.head = self.wrap_add(self.tail, len);
            }
        } else {
            // free is smaller than both head and tail,
            // this means we have to slowly "swap" the tail and the head.
            //
            // from: EFGHI...ABCD or HIJK.ABCDEFG
            // to:   ABCDEFGHI... or ABCDEFGHIJK.
            let mut left_edge: usize = 0;
            let mut right_edge: usize = self.tail;
            unsafe {
                // The general problem looks like this
                // GHIJKLM...ABCDEF - before any swaps
                // ABCDEFM...GHIJKL - after 1 pass of swaps
                // ABCDEFGHIJM...KL - swap until the left edge reaches the temp store
                //                  - then restart the algorithm with a new (smaller) store
                // Sometimes the temp store is reached when the right edge is at the end
                // of the buffer - this means we've hit the right order with fewer swaps!
                // E.g
                // EF..ABCD
                // ABCDEF.. - after four only swaps we've finished
                while left_edge < len && right_edge != cap {
                    let mut right_offset = 0;
                    for i in left_edge..right_edge {
                        right_offset = (i - left_edge) % (cap - right_edge);
                        let src: isize = (right_edge + right_offset) as isize;
                        ptr::swap(buf.add(i), buf.offset(src));
                    }
                    let n_ops = right_edge - left_edge;
                    left_edge += n_ops;
                    right_edge += right_offset + 1;
                }

                self.tail = 0;
                self.head = len;
            }
        }

        let tail = self.tail;
        let head = self.head;
        unsafe { RingSlices::ring_slices(self.buffer_as_mut_slice(), head, tail).0 }
    }
}

impl<T: Clone, A: Allocator> VecDeque<T, A> {
    /// Modifies the deque in-place so that `len()` is equal to new_len,
    /// either by removing excess elements from the back or by appending clones of `value`
    /// to the back.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::VecDeque;
    ///
    /// let mut buf = VecDeque::new();
    /// buf.push_back(5);
    /// buf.push_back(10);
    /// buf.push_back(15);
    /// assert_eq!(buf, [5, 10, 15]);
    ///
    /// buf.resize(2, 0);
    /// assert_eq!(buf, [5, 10]);
    ///
    /// buf.resize(5, 20);
    /// assert_eq!(buf, [5, 10, 20, 20, 20]);
    /// ```
    //#[stable(feature = "deque_extras", since = "1.16.0")]
    #[cfg(real_std)]
    pub fn resize(&mut self, new_len: usize, value: T) {
        self.resize_with(new_len, || value.clone());
    }
}

/// Returns the index in the underlying buffer for a given logical element index.
#[inline]
fn wrap_index(index: usize, size: usize) -> usize {
    // size is always a power of 2
    debug_assert!(size.is_power_of_two());
    index & (size - 1)
}

/// Calculate the number of elements left to be read in the buffer
#[inline]
fn count(tail: usize, head: usize, size: usize) -> usize {
    // size is always a power of 2
    (head.wrapping_sub(tail)) & (size - 1)
}
