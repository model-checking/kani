// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks Kani's support for the `itoa` crate
//! Currently fails with a spurious failure:
//! https://github.com/model-checking/kani/issues/2066

use itoa::{Buffer, Integer};
use std::fmt::Write;

fn check_itoa<T: kani::Arbitrary + Integer + std::fmt::Display>() {
    let input: T = kani::any();
    let mut buf = Buffer::new();
    let result = buf.format(input);
    let mut output = String::new();
    write!(&mut output, "{}", input);
    assert_eq!(result, &output);
}

/// Note: We ignore this harness for now due to a performance regression.
/// See <https://github.com/model-checking/kani/issues/2576> for more details.
#[kani::proof]
#[kani::unwind(10)]
fn check_signed() {
    check_itoa::<i8>();
}

#[kani::proof]
#[kani::unwind(10)]
fn check_unsigned() {
    check_itoa::<u8>();
}

fn main() {}
