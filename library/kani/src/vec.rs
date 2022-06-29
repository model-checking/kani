// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use crate::{any, Arbitrary};

/// Generates an arbitrary vector given a length.
pub fn any_vec<T, const VEC_LENGTH: usize>() -> Vec<T>
where
    T: Arbitrary,
{
    let boxed_any_slice: Box<[T; VEC_LENGTH]> = Box::new(any());
    <[T]>::into_vec(boxed_any_slice)
}
