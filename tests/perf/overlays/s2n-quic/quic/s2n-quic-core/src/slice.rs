// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use core::ops::{Deref, DerefMut};

pub mod deque;

/// Copies vectored slices from one slice into another
///
/// The number of copied items is limited by the minimum of the lengths of each of the slices.
///
/// Returns the number of entries that were copied
#[inline]
pub fn vectored_copy<A, B, T>(from: &[A], to: &mut [B]) -> usize
where
    A: Deref<Target = [T]>,
    B: Deref<Target = [T]> + DerefMut,
    T: Copy,
{
    zip_chunks(from, to, |a, b| {
        b.copy_from_slice(a);
    })
}

/// Zips entries from one slice to another
///
/// The number of copied items is limited by the minimum of the lengths of each of the slices.
///
/// Returns the number of entries that were processed
#[inline]
pub fn zip<A, At, B, Bt, F>(from: &[A], to: &mut [B], mut on_item: F) -> usize
where
    A: Deref<Target = [At]>,
    B: Deref<Target = [Bt]> + DerefMut,
    F: FnMut(&At, &mut Bt),
{
    zip_chunks(from, to, |a, b| {
        for (a, b) in a.iter().zip(b) {
            on_item(a, b);
        }
    })
}

/// Zips overlapping chunks from one slice to another
///
/// The number of copied items is limited by the minimum of the lengths of each of the slices.
///
/// Returns the number of entries that were processed
#[inline]
pub fn zip_chunks<A, At, B, Bt, F>(from: &[A], to: &mut [B], mut on_slice: F) -> usize
where
    A: Deref<Target = [At]>,
    B: Deref<Target = [Bt]> + DerefMut,
    F: FnMut(&[At], &mut [Bt]),
{
    let mut count = 0;

    let mut from_index = 0;
    let mut from_offset = 0;

    let mut to_index = 0;
    let mut to_offset = 0;

    // The compiler isn't smart enough to remove all of the bounds checks so we resort to
    // `get_unchecked`.
    //
    // https://godbolt.org/z/45cG1v

    // iterate until we reach one of the ends
    while from_index < from.len() && to_index < to.len() {
        let from = unsafe {
            // Safety: this length is already checked in the while condition
            debug_assert!(from.len() > from_index);
            from.get_unchecked(from_index)
        };

        let to = unsafe {
            // Safety: this length is already checked in the while condition
            debug_assert!(to.len() > to_index);
            to.get_unchecked_mut(to_index)
        };

        {
            // calculate the current views
            let from = unsafe {
                // Safety: the slice offsets are checked at the end of the while loop
                debug_assert!(from.len() >= from_offset);
                from.get_unchecked(from_offset..)
            };

            let to = unsafe {
                // Safety: the slice offsets are checked at the end of the while loop
                debug_assert!(to.len() >= to_offset);
                to.get_unchecked_mut(to_offset..)
            };

            let len = from.len().min(to.len());

            unsafe {
                // Safety: by using the min of the two lengths we will never exceed
                //         either slice's buffer
                debug_assert!(from.len() >= len);
                debug_assert!(to.len() >= len);

                let at = from.get_unchecked(..len);
                let bt = to.get_unchecked_mut(..len);

                on_slice(at, bt);
            }

            // increment the offsets
            from_offset += len;
            to_offset += len;
            count += len;
        }

        // check if the `from` is done
        if from.len() == from_offset {
            from_index += 1;
            from_offset = 0;
        }

        // check if the `to` is done
        if to.len() == to_offset {
            to_index += 1;
            to_offset = 0;
        }
    }

    count
}

/// Deduplicates elements in a slice
///
/// # Note
///
/// Items must be sorted before performing this function
#[inline]
pub fn partition_dedup<T>(slice: &mut [T]) -> (&mut [T], &mut [T])
where
    T: PartialEq,
{
    // TODO replace with
    // https://doc.rust-lang.org/std/primitive.slice.html#method.partition_dedup
    // when stable
    //
    // For now, we've just inlined their implementation

    let len = slice.len();
    if len <= 1 {
        return (slice, &mut []);
    }

    let ptr = slice.as_mut_ptr();
    let mut next_read: usize = 1;
    let mut next_write: usize = 1;

    // SAFETY: the `while` condition guarantees `next_read` and `next_write`
    // are less than `len`, thus are inside `self`. `prev_ptr_write` points to
    // one element before `ptr_write`, but `next_write` starts at 1, so
    // `prev_ptr_write` is never less than 0 and is inside the slice.
    // This fulfils the requirements for dereferencing `ptr_read`, `prev_ptr_write`
    // and `ptr_write`, and for using `ptr.add(next_read)`, `ptr.add(next_write - 1)`
    // and `prev_ptr_write.offset(1)`.
    //
    // `next_write` is also incremented at most once per loop at most meaning
    // no element is skipped when it may need to be swapped.
    //
    // `ptr_read` and `prev_ptr_write` never point to the same element. This
    // is required for `&mut *ptr_read`, `&mut *prev_ptr_write` to be safe.
    // The explanation is simply that `next_read >= next_write` is always true,
    // thus `next_read > next_write - 1` is too.
    unsafe {
        // Avoid bounds checks by using raw pointers.
        while next_read < len {
            let ptr_read = ptr.add(next_read);
            let prev_ptr_write = ptr.add(next_write - 1);
            if *ptr_read != *prev_ptr_write {
                if next_read != next_write {
                    let ptr_write = prev_ptr_write.add(1);
                    core::ptr::swap(ptr_read, ptr_write);
                }
                next_write += 1;
            }
            next_read += 1;
        }
    }

    slice.split_at_mut(next_write)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::InlineVec;
    use bolero::check;

    fn assert_eq_slices<A, B, T>(a: &[A], b: &[B])
    where
        A: Deref<Target = [T]>,
        B: Deref<Target = [T]>,
        T: PartialEq + core::fmt::Debug,
    {
        let a = a.iter().flat_map(|a| a.iter());
        let b = b.iter().flat_map(|b| b.iter());

        // make sure all of the values match
        //
        // Note: this doesn't use Iterator::eq, as the slice lengths may be different
        for (a, b) in a.zip(b) {
            assert_eq!(a, b);
        }
    }

    #[test]
    fn vectored_copy_test() {
        let from = [
            &[0][..],
            &[1, 2, 3][..],
            &[4, 5, 6, 7][..],
            &[][..],
            &[8, 9, 10, 11][..],
        ];

        for len in 0..6 {
            let mut to = vec![vec![0; 2]; len];
            let copied_len = vectored_copy(&from, &mut to);
            assert_eq!(copied_len, len * 2);
            assert_eq_slices(&from, &to);
        }
    }

    const LEN: usize = if cfg!(kani) { 2 } else { 32 };

    #[test]
    #[cfg_attr(kani, kani::proof, kani::unwind(5), kani::solver(kissat))]
    #[cfg_attr(miri, ignore)] // This test is too expensive for miri to complete in a reasonable amount of time
    fn vectored_copy_fuzz_test() {
        check!()
            .with_type::<(
                InlineVec<InlineVec<u8, LEN>, LEN>,
                InlineVec<InlineVec<u8, LEN>, LEN>,
            )>()
            .cloned()
            .for_each(|(from, mut to)| {
                vectored_copy(&from, &mut to);
                assert_eq_slices(&from, &to);
            })
    }
}
