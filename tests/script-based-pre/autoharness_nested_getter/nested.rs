// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Regression for https://github.com/model-checking/kani/issues/5011.

#[repr(C)]
pub struct Inner {
    pub count: u32,
}

impl Inner {
    #[inline(never)]
    pub fn valid(&self) -> bool {
        self.count > 0
    }
}

#[repr(C)]
pub struct Outer {
    pub decoy: u32,
    pub inner: Inner,
}

impl Outer {
    pub fn first(&self) {
        assert!(self.inner.valid());
    }

    pub fn second(&self) {
        assert!(self.inner.valid());
    }
}

pub fn make_valid() -> Outer {
    Outer { decoy: 0, inner: Inner { count: 1 } }
}

pub fn make_invalid() -> Outer {
    Outer { decoy: 1, inner: Inner { count: 0 } }
}

fn main() {
    let value = make_valid();
    value.first();
    value.second();
    assert!(!make_invalid().inner.valid());
}

// A multi-field receiver path must also be retained.
#[repr(C)]
pub struct Deep {
    pub decoy: u32,
    pub outer: Outer,
}

impl Deep {
    pub fn first(&self) {
        assert!(self.outer.inner.valid());
    }
    pub fn second(&self) {
        assert!(self.outer.inner.valid());
    }
}

pub fn make_deep_valid() -> Deep {
    Deep { decoy: 0, outer: make_valid() }
}

pub fn make_deep_invalid() -> Deep {
    Deep { decoy: 1, outer: make_invalid() }
}

// Receivers behind a `Box`, reference or raw pointer field: the getter's fields are not
// fields of the caller's struct, so nothing may be mined through them. Each `make_*_invalid`
// has `decoy != 0` and `inner.count == 0`; a wrong rebase would mine `decoy > 0` or
// `inner.count > 0` on the caller's layout and turn these into failures.
static ZERO: Inner = Inner { count: 0 };

#[repr(C)]
pub struct Boxed {
    pub decoy: u32,
    pub inner: Box<Inner>,
}

impl Boxed {
    pub fn first(&self) {
        assert!(self.inner.valid());
    }
    pub fn second(&self) {
        assert!(self.inner.valid());
    }
}

pub fn make_boxed_invalid() -> Boxed {
    Boxed { decoy: 1, inner: Box::new(Inner { count: 0 }) }
}

#[repr(C)]
pub struct Borrowed {
    pub decoy: u32,
    pub inner: &'static Inner,
}

impl Borrowed {
    pub fn first(&self) {
        assert!(self.inner.valid());
    }
    pub fn second(&self) {
        assert!(self.inner.valid());
    }
}

pub fn make_borrowed_invalid() -> Borrowed {
    Borrowed { decoy: 1, inner: &ZERO }
}

#[repr(C)]
pub struct Pointed {
    pub decoy: u32,
    pub inner: *const Inner,
}

impl Pointed {
    pub fn first(&self) {
        assert!(unsafe { (*self.inner).valid() });
    }
    pub fn second(&self) {
        assert!(unsafe { (*self.inner).valid() });
    }
}

pub fn make_pointed_invalid() -> Pointed {
    Pointed { decoy: 1, inner: &ZERO }
}
