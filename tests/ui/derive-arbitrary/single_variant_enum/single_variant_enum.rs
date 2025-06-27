// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that Kani can automatically derive Arbitrary enums.
//! An arbitrary enum should always generate a valid arbitrary variant.

extern crate kani;
use kani::cover;

#[derive(kani::Arbitrary)]
enum Simple {
    Empty,
}

#[kani::proof]
fn check_simple() {
    match kani::any::<Simple>() {
        Simple::Empty => cover!(),
    }
}

#[derive(kani::Arbitrary)]
enum WithArgs {
    Args(char),
}

#[kani::proof]
fn check_with_args() {
    match kani::any::<WithArgs>() {
        WithArgs::Args(c) => {
            assert!(c <= char::MAX);
            cover!(c == 'a');
            cover!(c == '1');
        }
    }
}

#[derive(kani::Arbitrary)]
enum WithNamedArgs {
    Args { flag: bool },
}

#[kani::proof]
fn check_with_named_args() {
    match kani::any::<WithNamedArgs>() {
        WithNamedArgs::Args { flag } => {
            cover!(flag as u8 == 0);
            cover!(flag as u8 == 1);
            assert!(flag as u8 == 0 || flag as u8 == 1);
        }
    }
}

#[derive(kani::Arbitrary)]
enum WithDiscriminant {
    Disc = 42,
}

#[kani::proof]
fn check_with_discriminant() {
    let v = kani::any::<WithDiscriminant>();
    let disc = v as i8;
    match v {
        WithDiscriminant::Disc => assert!(disc == 42),
    }
}
