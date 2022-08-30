// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check that users can implement Arbitrary to a simple data struct with Vec<>.
extern crate kani;
use kani::Arbitrary;

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
fn check_any() {
    let wrap: VecWrapper = kani::any();
    assert!(wrap.is_valid());
}
