// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that https://github.com/model-checking/kani/issues/2857 is
// fixed

#[kani::proof]
fn check_der_error() {
    let e = sec1::der::Error::incomplete(sec1::der::Length::ZERO);
    let _ = format!("{e:?}");
}

fn main() {}
