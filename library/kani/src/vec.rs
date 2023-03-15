// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use crate::{any, assume, Arbitrary};

/// Generates an arbitrary vector whose length is at most MAX_LENGTH.
pub fn any_vec<T, const MAX_LENGTH: usize>() -> Vec<T>
where
    T: Arbitrary,
    [(); std::mem::size_of::<[T; MAX_LENGTH]>()]:,
{
    let mut v = exact_vec::<T, MAX_LENGTH>();
    let real_length: usize = any();
    assume(real_length <= MAX_LENGTH);
    unsafe { v.set_len(real_length) };

    v
}

/// Generates an arbitrary vector that is exactly EXACT_LENGTH long.
pub fn exact_vec<T, const EXACT_LENGTH: usize>() -> Vec<T>
where
    T: Arbitrary,
    [(); std::mem::size_of::<[T; EXACT_LENGTH]>()]:,
{
    let boxed_array: Box<[T; EXACT_LENGTH]> = Box::new(any());
    <[T]>::into_vec(boxed_array)
}
