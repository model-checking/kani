// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::Arbitrary;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

/// Create an array with non-deterministic content and a non-deterministic
/// length. This API currently works with the Boogie backend only.
///
/// # Example:
///
/// ```
/// let arr = kani::array::any_array::<u32>();
/// if arr.len() > 2 {
///     kani::assume(arr[0] == 0);
///     kani::assume(arr[1] == arr[0]);
///     assert!(arr[1] == 0);
/// }
/// ```
#[inline(never)]
#[rustc_diagnostic_item = "KaniAnyArray"]
pub fn any_array<T>() -> Array<T>
where
    T: Arbitrary,
{
    #[allow(clippy::empty_loop)]
    loop {}
}

/// An array-like data structure that is intended for unbounded verification
pub struct Array<T> {
    _p: PhantomData<T>,
    len: usize,
}

impl<T> Array<T>
where
    T: Arbitrary,
{
    /// Get the length of the array
    #[inline(never)]
    #[rustc_diagnostic_item = "KaniAnyArrayLen"]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the array is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<T> Index<usize> for Array<T> {
    type Output = T;

    #[inline(never)]
    #[rustc_diagnostic_item = "KaniAnyArrayIndex"]
    fn index(&self, _index: usize) -> &Self::Output {
        #[allow(clippy::empty_loop)]
        loop {}
    }
}

impl<T> IndexMut<usize> for Array<T> {
    #[inline(never)]
    #[rustc_diagnostic_item = "KaniAnyArrayIndexMut"]
    fn index_mut(&mut self, _index: usize) -> &mut Self::Output {
        #[allow(clippy::empty_loop)]
        loop {}
    }
}
