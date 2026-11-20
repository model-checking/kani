// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers

#[kani::proof]
fn exists_assume_harness() {
    let j = kani::any();
    kani::assume(kani::exists!(|i in (2,4)| i == j));
    kani::assert(j == 3, "assume with exists");
}
