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

//! Arbitrary implementations for primitive types.

use crate::bool;

// TODO: implement the remaining types
// use crate::char;
// use crate::num::{f32, f64, i16, i32, i64, i8, isize, u16, u32, u64, u8, usize};
// #[cfg(not(target_arch = "wasm32"))]
// use crate::num::{i128, u128};

arbitrary!(bool);

// TODO: implement the remaining types
//arbitrary!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, bool);

// #[cfg(not(target_arch = "wasm32"))]
//arbitrary!(i128, u128);

// Note that for floating point types we limit the space since a lot of code
// isn't prepared for (and is not intended to be) things like NaN and infinity.

// arbitrary!(f32, f32::Any; {
//     f32::POSITIVE | f32::NEGATIVE | f32::ZERO | f32::SUBNORMAL | f32::NORMAL
// });
// arbitrary!(f64, f64::Any; {
//     f64::POSITIVE | f64::NEGATIVE | f64::ZERO | f64::SUBNORMAL | f64::NORMAL
// });

// arbitrary!(char, char::CharStrategy<'static>; char::any());
