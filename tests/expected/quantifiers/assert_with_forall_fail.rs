// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

extern crate kani;
use kani::kani_forall;

#[kani::proof]
fn forall_assert_harness() {
    let j = kani::any();
    kani::assume(j > 3);
    kani::assert(kani::forall!(|i in (2,5)| i < j ), "assertion with forall");
}
