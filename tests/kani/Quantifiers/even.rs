// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers

#[kani::proof]
fn quantifier_even_harness() {
    let j: usize = kani::any();
    kani::assume(j % 2 == 0 && j < 2000);
    kani::assert(kani::exists!(|i in (0, 1000)| i + i == j), "");
}
