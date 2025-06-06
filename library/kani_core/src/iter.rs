// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This macro generates implementations of the `KaniIntoIter` trait for various common types that are used in for loop.
//! We use this trait to overwrite the Rust IntoIter trait to reduce call stacks and avoid complicated loop invariant specifications,
//! while maintaining the semantic of the loop.


#[macro_export]
#[allow(clippy::crate_in_macro_def)]
macro_rules! generate_iter {
    () => {
        use core_path::slice::Iter;

        pub struct KaniIter<T> {
            pub iptr: *const T,
            pub len: usize,
            pub current_pos: usize,
        }

        impl<T> KaniIter<T> {
            pub fn new(iptr: *const T, len: usize) -> Self {
                KaniIter { iptr, len, current_pos: 0 }
            }
            pub fn next(&mut self) -> Option<&T> {
                if self.current_pos < self.len {
                    let elem = unsafe { &*self.iptr.wrapping_add(self.current_pos) };
                    self.current_pos += 1;
                    Some(elem)
                } else {
                    None
                }
            }
        }

        pub trait KaniIntoIter
        where
            Self: Sized,
        {
            type Item;
            fn kani_into_iter(self) -> (*const Self::Item, usize);
        }

        impl<T, const N: usize> KaniIntoIter for [T; N] {
            type Item = T;
            fn kani_into_iter(self) -> (*const Self::Item, usize) {
                (self.as_ptr(), N)
            }
        }

        impl<T> KaniIntoIter for Iter<'_, T> {
            type Item = T;
            fn kani_into_iter(self) -> (*const Self::Item, usize) {
                (self.as_slice().as_ptr(), self.len())
            }
        }

        impl<'a, T> KaniIntoIter for &'a [T] {
            type Item = T;
            fn kani_into_iter(self) -> (*const Self::Item, usize) {
                (self.as_ptr(), self.len())
            }
        }

        impl<'a, T> KaniIntoIter for &'a mut [T] {
            type Item = T;
            fn kani_into_iter(self) -> (*const Self::Item, usize) {
                (self.as_ptr(), self.len())
            }
        }

        impl<T> KaniIntoIter for Vec<T> {
            type Item = T;
            fn kani_into_iter(self) -> (*const Self::Item, usize) {
                let s = self.iter();
                (s.as_slice().as_ptr(), s.len())
            }
        }
    };
}
