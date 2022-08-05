//-
// Copyright 2017, 2018 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

//! Arbitrary implementations for tuples.

use crate::arbitrary::{any_with, Arbitrary};

macro_rules! impl_tuple {
    ($($typ: ident; $index:tt),*) => {
        impl<$($typ : Arbitrary),*> Arbitrary for ($($typ,)*) {
            type Parameters = ($($typ::Parameters,)*);
            type Strategy = ($($typ::Strategy,)*);
            fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
                #[allow(non_snake_case)]
                ($(any_with::<$typ>(args.$index)),*,)
            }
        }
    };
}

arbitrary!((); ());
impl_tuple!(T0; 0);
impl_tuple!(T0; 0, T1; 1);
impl_tuple!(T0; 0, T1; 1, T2; 2);
impl_tuple!(T0; 0, T1; 1, T2; 2, T3; 3);
impl_tuple!(T0; 0, T1; 1, T2; 2, T3; 3, T4; 4);
impl_tuple!(T0; 0, T1; 1, T2; 2, T3; 3, T4; 4, T5; 5);
impl_tuple!(T0; 0, T1; 1, T2; 2, T3; 3, T4; 4, T5; 5, T6; 6);
impl_tuple!(T0; 0, T1; 1, T2; 2, T3; 3, T4; 4, T5; 5, T6; 6, T7; 7);
impl_tuple!(T0; 0, T1; 1, T2; 2, T3; 3, T4; 4, T5; 5, T6; 6, T7; 7, T8; 8);
impl_tuple!(T0; 0, T1; 1, T2; 2, T3; 3, T4; 4, T5; 5, T6; 6, T7; 7, T8; 8, T9; 9);

