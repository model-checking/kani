//-
// Copyright 2017, 2018 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Arbitrary implementations for libstd.

/*
mod ascii;
*/
mod boxed;
mod cell;
//mod char;
mod cmp;
mod collections;
mod convert;
/*
mod env;
mod ffi;
*/
mod fmt;
mod fs;
mod hash;
#[cfg(feature = "unstable")]
mod heap;
/*
mod io;
mod iter;
mod marker;
mod mem;
mod net;
*/
mod num;
mod ops;
mod option;
mod panic;
//mod path;
mod rc;
/*
mod result;
mod str;
mod string;
pub use self::string::*;
mod sync;
mod thread;
*/
mod time;