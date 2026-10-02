// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --coverage -Zsource-coverage

// Regression for #4838: coverage must preserve spaces in source filenames.
#[path = "auxiliary/a a.rs"]
mod a;

#[kani::proof]
fn main() {
    a::c();
}
