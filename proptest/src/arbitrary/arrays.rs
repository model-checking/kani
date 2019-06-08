//-
// Copyright 2017, 2018 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Arbitrary implementations for arrays.

use crate::arbitrary::{any_with, Arbitrary};
use crate::array::UniformArrayStrategy;

macro_rules! array {
    ($($n: expr),*) => { $(
        impl<A: Arbitrary> Arbitrary for [A; $n] {
            type Parameters = A::Parameters;
            type Strategy = UniformArrayStrategy<A::Strategy, [A; $n]>;
            fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
                let base = any_with::<A>(args);
                UniformArrayStrategy::new(base)
            }
        }
    )* };
}

array!(
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21,
    22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32
);

#[cfg(test)]
mod test {
    no_panic_test!(
        array_16 => [u8; 16]
    );
}
