// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This macro generates implementations of the `KaniIntoIter` trait for various common types that are used in for loop.
//! We use this trait to overwrite the Rust IntoIter trait to reduce call stacks and avoid complicated loop invariant specifications,
//! while maintaining the semantic of the loop.

use crate::{KaniIntoIter, KaniPtrIter};

impl<T: Copy> KaniIntoIter for Vec<T> {
    type Iter = KaniPtrIter<T>;
    fn kani_into_iter(self) -> Self::Iter {
        let s = self.iter();
        KaniPtrIter::new(s.as_slice().as_ptr(), s.len())
    }
}
