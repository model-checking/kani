// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

extern crate kani;
use kani::kani_exists;

#[kani::proof]
fn quantifier_even_harness() {
    let j: isize = kani::any();
    kani::assume(j % 2 == 0);
    kani::assert(kani::exists!(|i in (-1000, 1000)| i + i == j), "");
}
