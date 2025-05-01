// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn forall_assert_harness() {
    let j = kani::any();
    kani::assume(j > 5);
    kani::assert(kani::forall!(|i in (2,5)| i < j ), "assertion with forall");
}
