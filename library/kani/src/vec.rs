// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#[allow(unused_imports)]
use crate::{any, any_where, Arbitrary};

/// Generates an arbitrary vector whose length is at most MAX_LENGTH.
#[cfg(kani_sysroot)]
pub fn any_vec<T, const MAX_LENGTH: usize>() -> Vec<T>
where
    T: Arbitrary,
    [(); std::mem::size_of::<[T; MAX_LENGTH]>()]:,
{
    let real_length: usize = any_where(|sz| *sz <= MAX_LENGTH);
    match real_length {
        0 => vec![],
        exact if exact == MAX_LENGTH => exact_vec::<T, MAX_LENGTH>(),
        _ => {
            let mut any_vec = exact_vec::<T, MAX_LENGTH>();
            any_vec.truncate(real_length);
            any_vec.shrink_to_fit();
            assert!(any_vec.capacity() == any_vec.len());
            any_vec
        }
    }
}

/// Generates an arbitrary vector that is exactly EXACT_LENGTH long.
#[cfg(kani_sysroot)]
pub fn exact_vec<T, const EXACT_LENGTH: usize>() -> Vec<T>
where
    T: Arbitrary,
    [(); std::mem::size_of::<[T; EXACT_LENGTH]>()]:,
{
    let boxed_array: Box<[T; EXACT_LENGTH]> = Box::new(any());
    <[T]>::into_vec(boxed_array)
}
