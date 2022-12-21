// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that Kani can automatically derive Arbitrary enums.
//! An arbitrary enum should always generate a valid arbitrary variant.

extern crate kani;
use kani::cover;

#[derive(kani::Arbitrary)]
enum Wrapper {
    Empty,
    Bool(bool),
    Char { c: char },
}

#[kani::proof]
fn check_enum_wrapper() {
    match kani::any::<Wrapper>() {
        Wrapper::Empty => cover!(),
        Wrapper::Bool(b) => {
            cover!(b as u8 == 0);
            cover!(b as u8 == 1);
            assert!(b as u8 == 0 || b as u8 == 1);
        }
        Wrapper::Char { c } => {
            assert!(c <= char::MAX);
            cover!(c == 'a');
            cover!(c == '1');
        }
    }
}

#[derive(kani::Arbitrary, Copy, Clone)]
enum Comparison {
    Less = -1,
    Equal = 0,
    Greater = 1,
}

#[kani::proof]
fn check_comparison() {
    let comp: Comparison = kani::any();
    let disc = comp as i8;
    assert!(disc >= -1 && disc <= 1);
    match comp {
        Comparison::Less => assert!(disc == -1),
        Comparison::Equal => assert!(disc == 0),
        Comparison::Greater => assert!(disc == 1),
    }
}
