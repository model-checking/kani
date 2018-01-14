//-
// Copyright 2017 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Provides `Rejection`, the reason why something, such
//! as a generated value, was rejected.

use std::borrow::Borrow;
use std::fmt;
use std::ops::Deref;
use std::borrow::Cow;

/// The reason for why something, such as a generated value, was rejected.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rejection(Cow<'static, str>);

impl From<&'static str> for Rejection {
    fn from(s: &'static str) -> Self {
        Rejection(s.into())
    }
}

impl From<String> for Rejection {
    fn from(s: String) -> Self {
        Rejection(s.into())
    }
}

impl From<Box<str>> for Rejection {
    fn from(s: Box<str>) -> Self {
        Rejection(String::from(s).into())
    }
}

impl Deref for Rejection {
    type Target = str;
    fn deref(&self) -> &Self::Target { self.0.deref() }
}

impl AsRef<str> for Rejection {
    fn as_ref(&self) -> &str { &*self }
}

impl Borrow<str> for Rejection {
    fn borrow(&self) -> &str { &*self }
}

impl fmt::Display for Rejection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self.as_ref(), f)
    }
}
