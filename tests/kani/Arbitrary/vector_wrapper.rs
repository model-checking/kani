// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check that the Invariant implementation for Option respect underlying types invariant.
#![cfg_attr(kani, feature(min_specialization))]

extern crate kani;
use kani::{Arbitrary, Invariant};

// Dummy wrappar that keeps track of the vector size.
struct VecWrapper {
    has_data: bool,
    data: Vec<u8>,
}

impl VecWrapper {
    fn new() -> Self {
        VecWrapper { has_data: false, data: Vec::new() }
    }

    fn from(buf: &[u8]) -> Self {
        VecWrapper { has_data: true, data: Vec::from(buf) }
    }
}

unsafe impl Invariant for VecWrapper {
    fn is_valid(&self) -> bool {
        self.has_data ^ self.data.is_empty()
    }
}

impl Arbitrary for VecWrapper {
    fn any() -> Self {
        if kani::any() { VecWrapper::new() } else { VecWrapper::from(&[kani::any(), kani::any()]) }
    }
}

#[kani::proof]
fn check() {
    let wrap: VecWrapper = kani::any();
    assert!(wrap.is_valid());
}
