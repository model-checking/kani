// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn exists_assert_harness() {
    let j = kani::any();
    kani::assume(j > 1);
    kani::assert(kani::exists!(|i in (3,5)| i < j ), "assertion with exists");
}
