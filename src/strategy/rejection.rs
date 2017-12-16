//-
// Copyright 2017 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Provides `Rejection`, a compact representation of why something, such
//! as a generated value, was rejected.

use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::Rc;

use self::Rejection::*;

/// The reason for why something, such as a generated value, was rejected.
///
/// # Representation
/// 
/// This type is currently representationally equivalent to:
///
/// ```rust
/// use std::rc::Rc;
/// 
/// pub enum Rejection {
///     Borrowed(&'static str),
///     BoxOwned(Box<str>),
///     RcOwned(Rc<str>),
/// }
///
/// # fn main() {
/// # use std::mem::size_of;
/// assert_eq!(size_of::<Rejection>(), 3 * size_of::<usize>());
/// # }
/// ```
///
/// This is a compact and essentially copy-on-write representation while also
/// allowing you not to allocate if the rejection message is not dynamic in
/// nature. This is most efficient and should be used when possible. If you
/// are allocating and then cloning the allocated `Rejection`, then you should
/// prefer the `Rejection::RcOwned` variant as it will only bump the reference
/// count and not actually do any new heap allocation.
pub enum Rejection {
    /// The borrowed representation - this is the most efficient if you for
    /// example have a string in the data section of your program.
    Borrowed(&'static str),
    /// A reference counted shared string, use this if you have dynamically
    /// generated content that is used more than once.
    RcOwned(Rc<str>),
    /// An owned boxed string, use this if you have a dynamicly generated
    /// string that is only used once. If used more than once, instead use
    /// `RcOwned`.
    BoxOwned(Box<str>),
}

/// Constructs and returns a [`Rejection`] based on the input type.
/// See the `From<T> for Rejection` implementations for details.
///
/// # Example
///
/// ```rust
/// fn main() {
///     use proptest::strategy::reject;
///     let reason = format!("The value {:?} was too much!", 100);
///     let reject = reject(reason);
///     println!("{:?}", reject);
/// }
/// ```
///
/// [`Rejection`]: enum.Rejection.html
pub fn reject<S: Into<Rejection>>(string: S) -> Rejection {
    string.into()
}

impl Rejection {
    /// Converts this `Rejection` into either an owned one backed by `Rc`
    /// or keeps the rejection as-is if it was in a borrowed form.
    pub fn for_reuse(self) -> Self {
        match self {
            Borrowed(s) => Borrowed(s.into()),
            BoxOwned(s) => RcOwned(s.into()),
            RcOwned(s)  => RcOwned(s.clone()),
        }
    }

    /// Converts this `Rejection` into an owned one backed by `Rc`.
    pub fn to_shared(self) -> Self {
        match self {
            Borrowed(s) => RcOwned(s.into()),
            BoxOwned(s) => RcOwned(s.into()),
            RcOwned(s)  => RcOwned(s.clone()),
        }
    }

    /// Converts this `Rejection` into an owned one backed by `Box`.
    pub fn to_owned(self) -> Self {
        match self {
            Borrowed(s) => BoxOwned(s.into()),
            BoxOwned(s) => BoxOwned(s),
            RcOwned(s)  => BoxOwned((&*s).into()),
        }
    }
}

impl From<&'static str> for Rejection {
    fn from(s: &'static str) -> Self {
        Borrowed(s)
    }
}

impl From<String> for Rejection {
    fn from(s: String) -> Self {
        BoxOwned(s.into())
    }
}

impl From<Box<str>> for Rejection {
    fn from(s: Box<str>) -> Self {
        BoxOwned(s)
    }
}

impl From<Rc<str>> for Rejection {
    fn from(s: Rc<str>) -> Self {
        RcOwned(s)
    }
}

impl Deref for Rejection {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        match *self {
            Borrowed(s) => s,
            BoxOwned(ref s) => &*s,
            RcOwned(ref s)  => &*s,
        }
    }
}

impl Clone for Rejection {
    fn clone(&self) -> Self {
        match *self {
            Borrowed(ref s) => Borrowed(s),
            BoxOwned(ref s) => BoxOwned(s.clone()),
            RcOwned(ref s)  => RcOwned(s.clone()),
        }
    }
}

impl fmt::Debug for Rejection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self.as_ref(), f)
    }
}

impl fmt::Display for Rejection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self.as_ref(), f)
    }
}

impl AsRef<str> for Rejection {
    fn as_ref(&self) -> &str { &*self }
}

impl Borrow<str> for Rejection {
    fn borrow(&self) -> &str { &*self }
}

impl<X: AsRef<str>> PartialEq<X> for Rejection {
    fn eq(&self, rhs: &X) -> bool { self.as_ref() == rhs.as_ref() }
}

impl Eq for Rejection {}

impl<X: AsRef<str>> PartialOrd<X> for Rejection {
    fn partial_cmp(&self, rhs: &X) -> Option<Ordering> {
        self.as_ref().partial_cmp(rhs.as_ref())
    }
}

impl Ord for Rejection {
    fn cmp(&self, rhs: &Self) -> Ordering { self.as_ref().cmp(rhs.as_ref()) }
}

impl Hash for Rejection {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state)
    }
}