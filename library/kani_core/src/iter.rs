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

        macro_rules! generate_impl_range {
            ($t:ty) => {
                impl KaniIter for Range<$t> {
                    type Item = $t;
                    fn indexing(&self, i: usize) -> Self::Item {
                        self.start + i as $t
                    }
                    fn first(&self) -> Self::Item {
                        self.start
                    }
                    fn assumption(&self) -> bool {
                        true
                    }
                    fn len(&self) -> usize {
                        (self.end - self.start) as usize
                    }
                }
            };
        }

        generate_impl_range!(i8);
        generate_impl_range!(i16);
        generate_impl_range!(i32);
        generate_impl_range!(i64);
        generate_impl_range!(isize);
        generate_impl_range!(u8);
        generate_impl_range!(u16);
        generate_impl_range!(u32);
        generate_impl_range!(u64);
        generate_impl_range!(usize);

        pub struct KaniStepBy<I: KaniIter> {
            iter: I,
            step: usize,
        }

        impl<I: KaniIter> KaniStepBy<I> {
            pub fn new(iter: I, step: usize) -> Self {
                KaniStepBy { iter, step }
            }
        }

        impl<I: KaniIter> KaniIter for KaniStepBy<I> {
            type Item = I::Item;

            fn indexing(&self, i: usize) -> Self::Item {
                self.iter.indexing(i * self.step)
            }

            fn first(&self) -> Self::Item {
                self.iter.first()
            }

            fn assumption(&self) -> bool {
                self.iter.assumption()
            }
            fn len(&self) -> usize {
                self.iter.len().div_ceil(self.step)
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

        impl KaniIntoIter for Range<i32> {
            type Iter = Range<i32>;
            fn kani_into_iter(self) -> Self::Iter {
                self
            }
        }

        impl<I: KaniIntoIter> KaniIntoIter for StepBy<I> {
            type Iter = KaniStepBy<I::Iter>;
            fn kani_into_iter(self) -> Self::Iter {
                struct StepByLayout<T> {
                    iter: T,
                    step_minus_one: usize,
                    first_take: bool,
                }
                let ptr = &self as *const StepBy<I> as *const StepByLayout<I>;
                let step = unsafe { (*ptr).step_minus_one + 1 };
                let iter = unsafe { core::ptr::read(&(*ptr).iter) };
                let kaniiter = iter.kani_into_iter();
                KaniStepBy::new(kaniiter, step)
            }
        }
    };
}
