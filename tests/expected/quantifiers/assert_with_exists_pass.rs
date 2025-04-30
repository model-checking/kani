// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

extern crate kani;
use kani::kani_exists;

#[kani::proof]
fn exists_assert_harness() {
    let j = kani::any();
    kani::assume(j > 2);
    kani::assert(kani::exists!(|i in (2,5)| i < j ), "assertion with exists");
}
