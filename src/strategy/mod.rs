//-
// Copyright 2017 Jason Lingle
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Defines the core traits used by Proptest.

mod traits;
mod map;
mod filter;
mod flatten;
mod unions;
mod recursive;

pub use self::traits::*;
pub use self::map::*;
pub use self::filter::*;
pub use self::flatten::*;
pub use self::unions::*;
pub use self::recursive::*;

pub mod statics;
