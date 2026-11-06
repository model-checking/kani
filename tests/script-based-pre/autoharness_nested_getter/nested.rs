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
