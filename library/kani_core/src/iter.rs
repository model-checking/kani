// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This macro generates implementations of the `KaniIntoIter` trait for various common types that are used in `for` loops.
//! We use this trait to overwrite the Rust IntoIter trait to reduce call stacks and avoid complicated loop invariant specifications,
//! while maintaining the semantics of the loop.

#[macro_export]
#[allow(clippy::crate_in_macro_def)]
macro_rules! generate_iter {
    () => {
        use core::iter::StepBy;
        use core_path::cmp::min;
        use core_path::iter::{Chain, Map, Zip};
        use core_path::mem as stdmem;
        use core_path::ops::Range;
        use core_path::slice::Iter;

        pub trait KaniIter
        where
            Self: Sized,
        {
            type Item;
            fn indexing(&self, i: usize) -> Self::Item;
            fn first(&self) -> Self::Item;
            fn assumption(&self) -> bool;
            fn len(&self) -> usize;
        }

        pub struct KaniPtrIter<T: Copy> {
            pub ptr: *const T,
            pub len: usize,
        }

        impl<T: Copy> KaniPtrIter<T> {
            pub fn new(ptr: *const T, len: usize) -> Self {
                KaniPtrIter { ptr, len }
            }
        }

        impl<T: Copy> KaniIter for KaniPtrIter<T> {
            type Item = T;
            fn indexing(&self, i: usize) -> Self::Item {
                unsafe { *self.ptr.wrapping_add(i) }
            }
            fn first(&self) -> Self::Item {
                unsafe { *self.ptr }
            }
            fn assumption(&self) -> bool {
                unsafe { mem::is_allocated(self.ptr as *const (), self.len) }
            }
            fn len(&self) -> usize {
                self.len
            }
        }

        pub struct KaniRefIter<'a, T: Copy> {
            pub ptr: *const T,
            pub len: usize,
            _marker: PhantomData<&'a T>,
        }

        impl<'a, T: Copy> KaniRefIter<'a, T> {
            pub fn new(ptr: *const T, len: usize) -> Self {
                KaniRefIter { ptr, len, _marker: PhantomData }
            }
        }

        impl<'a, T: Copy> KaniIter for KaniRefIter<'a, T> {
            type Item = &'a T;
            fn indexing(&self, i: usize) -> Self::Item {
                unsafe { &*self.ptr.wrapping_add(i) }
            }
            fn first(&self) -> Self::Item {
                unsafe { &*self.ptr }
            }
            fn assumption(&self) -> bool {
                unsafe { mem::is_allocated(self.ptr as *const (), self.len) }
            }
            fn len(&self) -> usize {
                self.len
            }
        }

        pub trait KaniIntoIter
        where
            Self: Sized,
        {
            type Iter: KaniIter;
            fn kani_into_iter(self) -> Self::Iter;
        }

        impl<T: Copy, const N: usize> KaniIntoIter for [T; N] {
            type Iter = KaniPtrIter<T>;
            fn kani_into_iter(self) -> Self::Iter {
                KaniPtrIter::new(self.as_ptr(), N)
            }
        }

        impl<'a, T: Copy> KaniIntoIter for &'a [T] {
            type Iter = KaniPtrIter<T>;
            fn kani_into_iter(self) -> Self::Iter {
                KaniPtrIter::new(self.as_ptr(), self.len())
            }
        }

        impl<'a, T: Copy> KaniIntoIter for &'a mut [T] {
            type Iter = KaniPtrIter<T>;
            fn kani_into_iter(self) -> Self::Iter {
                KaniPtrIter::new(self.as_ptr(), self.len())
            }
        }

        impl<'a, T: Copy> KaniIntoIter for Iter<'a, T> {
            type Iter = KaniRefIter<'a, T>;
            fn kani_into_iter(self) -> Self::Iter {
                KaniRefIter::new(self.as_slice().as_ptr(), self.len())
            }
        }
    };
}
