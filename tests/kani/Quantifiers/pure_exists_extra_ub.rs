// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers -Z valid-value-checks -Z uninit-checks --solver cadical

#[kani::proof]
fn witness_exists() {
    let witness: usize = kani::any();
    kani::assume(witness < 2);
    assert!(kani::exists!(|i in (0, 2)| i == witness));
}

#[kani::proof]
#[kani::should_panic]
fn witness_outside_range() {
    let witness: usize = kani::any();
    kani::assume(witness >= 2);
    assert!(kani::exists!(|i in (0, 2)| i == witness));
}
