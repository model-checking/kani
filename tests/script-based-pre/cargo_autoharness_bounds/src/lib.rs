// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that the bounds autoharness generates bounded arguments with are configurable. Each
// function asserts that its argument never exceeds the bound the test run configures (2), and
// covers that bound; with the defaults (16 for slices, 4 for strings and BoundedArbitrary
// values) the assertions would fail. Note that slices and `Vec`s of primitive elements take
// the unbounded generation path, so they are unaffected by these options.

// TEST NOTE: should PASS, and the cover check must be SATISFIED: slices of an element type
// whose Arbitrary implementation the compiler derives are generated element-wise, up to
// --slice-bound elements and no more.
pub struct Element {
    pub v: u8,
}

pub fn slice_bound(xs: &[Element]) {
    assert!(xs.len() <= 2);
    kani::cover!(xs.len() == 2, "slice at the configured bound");
}

// TEST NOTE: should PASS, and the cover check must be SATISFIED: strings are generated up to
// --string-bound bytes and no more.
pub fn string_bound(s: &str) {
    assert!(s.len() <= 2);
    kani::cover!(s.len() == 2, "string at the configured bound");
}

// TEST NOTE: should PASS, and the cover check must be SATISFIED: values of a type deriving
// BoundedArbitrary are generated up to --bounded-arbitrary-bound and no more.
#[derive(kani::BoundedArbitrary)]
pub struct Payload {
    #[bounded]
    bytes: Vec<u8>,
}

pub fn bounded_arbitrary_bound(p: Payload) {
    assert!(p.bytes.len() <= 2);
    kani::cover!(p.bytes.len() == 2, "BoundedArbitrary value at the configured bound");
}
