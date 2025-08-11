// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This macro generates implementations of the `KaniIntoIter` trait for various common types that are used in `for` loops.
//! We use this trait to overwrite the Rust IntoIter trait to reduce call stacks and avoid complicated loop invariant specifications,
//! while maintaining the semantics of the loop (see https://model-checking.github.io/kani/rfc/rfcs/0012-loop-contracts.html).

/*
The main idea is to override the Rust function into_iter by kani_into_iter(), so that instead of rewritting the for-loop as

let mut kani_iter = a.into_iter();
loop {
  match kani_iter.next() {
    Some (i) => {
      ...  //loop body
      }
    None => {break; }
  }
}

we rewrite it as:

let kani_iter = kani::kani_into_iter(a);
let mut kani_index = 0;
#[kani::loop_invariant(...)]
while (kani_index < kani_iter_len) {
  i = kani_iter.indexing(kani_index);
  // loop_body
  kani_index += 1;
}

We ensure the semantic by ensuring that the value of `i` is the same in each iteration of the loop for both versions,
while keeping the variable kani_iter immutable in our version.
In other word, we replace the next function with the indexing function to get the current item of the Iterator.

We overwrite the returned type R of Rust into_iter() for a type T by a corresponding Kani internal type K which implements KaniIter trait as follows:

T: array ,  R: IntoIter,  K: KaniPtrIter
T: slice ,  R: IntoIter,  K: KaniPtrIter
T: Iter  ,  R: Iter,      K: KaniRefIter
T: Range ,  R: Range,  K: Range
T: StepBy ,  R: StepBy,  K: KaniStepBy
T: Chain ,  R: Chain,  K: KaniChainIter
T: Zip ,  R: Zip,  K: KaniZipIter
T: Map ,  R: Map,  K: KaniMapIter
T: Enumerate ,  R: Enumerate,  K: KaniEnumerateIter
*/

#[macro_export]
#[allow(clippy::crate_in_macro_def)]
macro_rules! generate_iter {
    () => {
        use core::iter::StepBy;
        use core_path::cmp::min;
        use core_path::iter::{Chain, Enumerate, Map, Rev, Take, Zip};
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
                //SAFETY: this call is safe as Rust compiler will complain if we write a for-loop for initnitialized object
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
                //SAFETY: this call is safe as Rust compiler will complain if we write a for-loop for uninitialized object
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

        pub struct KaniChainIter<I: KaniIter> {
            pub iter1: I,
            pub iter2: I,
        }

        impl<I: KaniIter> KaniChainIter<I> {
            pub fn new(iter1: I, iter2: I) -> Self {
                KaniChainIter { iter1, iter2 }
            }
        }

        impl<I: KaniIter> KaniIter for KaniChainIter<I> {
            type Item = I::Item;
            fn indexing(&self, i: usize) -> Self::Item {
                if i < self.iter1.len() {
                    self.iter1.indexing(i)
                } else {
                    self.iter2.indexing(i - self.iter1.len())
                }
            }
            fn first(&self) -> Self::Item {
                if self.iter1.len() > 0 { self.iter1.first() } else { self.iter2.first() }
            }
            fn assumption(&self) -> bool {
                self.iter1.assumption() || self.iter2.assumption()
            }
            fn len(&self) -> usize {
                self.iter1.len() + self.iter2.len()
            }
        }

        pub struct KaniZipIter<I1: KaniIter, I2: KaniIter> {
            pub iter1: I1,
            pub iter2: I2,
        }

        impl<I1: KaniIter, I2: KaniIter> KaniZipIter<I1, I2> {
            pub fn new(iter1: I1, iter2: I2) -> Self {
                KaniZipIter { iter1, iter2 }
            }
        }

        impl<I1: KaniIter, I2: KaniIter> KaniIter for KaniZipIter<I1, I2> {
            type Item = (I1::Item, I2::Item);
            fn indexing(&self, i: usize) -> Self::Item {
                (self.iter1.indexing(i), self.iter2.indexing(i))
            }
            fn first(&self) -> Self::Item {
                (self.iter1.first(), self.iter2.first())
            }
            fn assumption(&self) -> bool {
                self.iter1.assumption() && self.iter2.assumption()
            }
            fn len(&self) -> usize {
                min(self.iter1.len(), self.iter2.len())
            }
        }

        pub struct KaniMapIter<I: KaniIter, F> {
            pub iter: I,
            pub map: F,
        }

        impl<I: KaniIter, F> KaniMapIter<I, F> {
            pub fn new(iter: I, map: F) -> Self {
                KaniMapIter { iter, map }
            }
        }

        impl<B, I: KaniIter, F> KaniIter for KaniMapIter<I, F>
        where
            F: FnMut(I::Item) -> B + Copy,
        {
            type Item = B;
            fn indexing(&self, i: usize) -> Self::Item {
                let item = self.iter.indexing(i);
                let map_ptr = &self.map as *const F as *mut F;
                unsafe { (*map_ptr)(item) }
            }
            fn first(&self) -> Self::Item {
                let item = self.iter.first();
                let map_ptr = &self.map as *const F as *mut F;
                unsafe { (*map_ptr)(item) }
            }
            fn assumption(&self) -> bool {
                self.iter.assumption()
            }
            fn len(&self) -> usize {
                self.iter.len()
            }
        }

        pub struct KaniEnumerateIter<I: KaniIter> {
            pub iter: I,
        }

        impl<I: KaniIter> KaniEnumerateIter<I> {
            pub fn new(iter: I) -> Self {
                KaniEnumerateIter { iter }
            }
        }

        impl<I: KaniIter> KaniIter for KaniEnumerateIter<I> {
            type Item = (usize, I::Item);
            fn indexing(&self, i: usize) -> Self::Item {
                let item = self.iter.indexing(i);
                (i, item)
            }
            fn first(&self) -> Self::Item {
                let item = self.iter.first();
                (0, item)
            }
            fn assumption(&self) -> bool {
                self.iter.assumption()
            }
            fn len(&self) -> usize {
                self.iter.len()
            }
        }

        pub struct KaniTakeIter<I: KaniIter> {
            pub iter: I,
            pub n: usize,
        }

        impl<I: KaniIter> KaniTakeIter<I> {
            pub fn new(iter: I, n: usize) -> Self {
                KaniTakeIter { iter, n }
            }
        }

        impl<I: KaniIter> KaniIter for KaniTakeIter<I> {
            type Item = I::Item;
            fn indexing(&self, i: usize) -> Self::Item {
                //assert!(i < self.n && i < self.iter.len());
                self.iter.indexing(i)
            }
            fn first(&self) -> Self::Item {
                self.iter.first()
            }
            fn assumption(&self) -> bool {
                self.iter.assumption()
            }
            fn len(&self) -> usize {
                min(self.iter.len(), self.n)
            }
        }

        pub struct KaniRevIter<I: KaniIter> {
            pub iter: I,
        }

        impl<I: KaniIter> KaniRevIter<I> {
            pub fn new(iter: I) -> Self {
                KaniRevIter { iter }
            }
        }

        impl<I: KaniIter> KaniIter for KaniRevIter<I> {
            type Item = I::Item;
            fn indexing(&self, i: usize) -> Self::Item {
                //assert!(i < self.n && i < self.iter.len());
                self.iter.indexing(self.iter.len() - 1 - i)
            }
            fn first(&self) -> Self::Item {
                self.iter.indexing(self.iter.len() - 1)
            }
            fn assumption(&self) -> bool {
                self.iter.assumption()
            }
            fn len(&self) -> usize {
                self.iter.len()
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

        macro_rules! generate_impl_KaniIter_range {
            ($t:ty) => {
                impl KaniIntoIter for Range<$t> {
                    type Iter = Range<$t>;
                    fn kani_into_iter(self) -> Self::Iter {
                        self
                    }
                }
            };
        }

        generate_impl_KaniIter_range!(i8);
        generate_impl_KaniIter_range!(i16);
        generate_impl_KaniIter_range!(i32);
        generate_impl_KaniIter_range!(i64);
        generate_impl_KaniIter_range!(isize);
        generate_impl_KaniIter_range!(u8);
        generate_impl_KaniIter_range!(u16);
        generate_impl_KaniIter_range!(u32);
        generate_impl_KaniIter_range!(u64);
        generate_impl_KaniIter_range!(usize);

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

        impl<I: KaniIntoIter + Clone> KaniIntoIter for Chain<I, I> {
            type Iter = KaniChainIter<I::Iter>;
            fn kani_into_iter(self) -> Self::Iter {
                struct ChainLayout<I> {
                    a: Option<I>,
                    b: Option<I>,
                }
                let ptr = &self as *const Chain<I, I> as *const ChainLayout<I>;
                let iter1 = unsafe { (*ptr).a.clone().unwrap().kani_into_iter() };
                let iter2 = unsafe { (*ptr).b.clone().unwrap().kani_into_iter() };
                KaniChainIter::new(iter1, iter2)
            }
        }

        impl<I1: KaniIntoIter + Clone, I2: KaniIntoIter + Clone> KaniIntoIter for Zip<I1, I2> {
            type Iter = KaniZipIter<I1::Iter, I2::Iter>;
            fn kani_into_iter(self) -> Self::Iter {
                struct ZipLayout<I1, I2> {
                    a: I1,
                    b: I2,
                    index: usize,
                    len: usize,
                    a_len: usize,
                }
                let ptr = &self as *const Zip<I1, I2> as *const ZipLayout<I1, I2>;
                let iter1 = unsafe { (*ptr).a.clone().kani_into_iter() };
                let iter2 = unsafe { (*ptr).b.clone().kani_into_iter() };
                KaniZipIter::new(iter1, iter2)
            }
        }

        impl<B, I: KaniIntoIter + Clone, F> KaniIntoIter for Map<I, F>
        where
            <<I as KaniIntoIter>::Iter as KaniIter>::Item: Clone,
            F: FnMut(<<I as KaniIntoIter>::Iter as KaniIter>::Item) -> B + Copy,
        {
            type Iter = KaniMapIter<I::Iter, F>;
            fn kani_into_iter(self) -> Self::Iter {
                struct MapLayout<I, F> {
                    iter: I,
                    map: F,
                }
                let ptr = &self as *const Map<I, F> as *const MapLayout<I, F>;
                let iter = unsafe { (*ptr).iter.clone().kani_into_iter() };
                let map = unsafe { (*ptr).map };
                KaniMapIter::new(iter, map)
            }
        }

        impl<I: KaniIntoIter + Clone> KaniIntoIter for Enumerate<I> {
            type Iter = KaniEnumerateIter<I::Iter>;
            fn kani_into_iter(self) -> Self::Iter {
                struct EnumerateLayout<I> {
                    iter: I,
                    count: usize,
                }
                let ptr = &self as *const Enumerate<I> as *const EnumerateLayout<I>;
                let iter = unsafe { (*ptr).iter.clone().kani_into_iter() };
                KaniEnumerateIter::new(iter)
            }
        }

        impl<I: KaniIntoIter + Clone> KaniIntoIter for Take<I> {
            type Iter = KaniTakeIter<I::Iter>;
            fn kani_into_iter(self) -> Self::Iter {
                struct TakeLayout<I> {
                    iter: I,
                    n: usize,
                }
                let ptr = &self as *const Take<I> as *const TakeLayout<I>;
                let iter = unsafe { (*ptr).iter.clone().kani_into_iter() };
                let n = unsafe { (*ptr).n };
                KaniTakeIter::new(iter, n)
            }
        }

        impl<I: KaniIntoIter + Clone> KaniIntoIter for Rev<I> {
            type Iter = KaniRevIter<I::Iter>;
            fn kani_into_iter(self) -> Self::Iter {
                struct RevLayout<I> {
                    iter: I,
                }
                let ptr = &self as *const Rev<I> as *const RevLayout<I>;
                let iter = unsafe { (*ptr).iter.clone().kani_into_iter() };
                KaniRevIter::new(iter)
            }
        }
    };
}
