// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers -Z valid-value-checks -Z uninit-checks --solver cadical

#[inline(never)]
fn identity(value: bool) -> bool {
    value
}

#[kani::proof]
fn valid_helper() {
    let enabled: bool = kani::any();
    kani::assume(enabled);
    assert!(kani::internal::kani_forall(0, 2, move |i: usize| identity(enabled) & (i < 2)));
}

#[kani::proof]
#[kani::should_panic]
fn false_helper_predicate() {
    let enabled: bool = kani::any();
    kani::assume(!enabled);
    assert!(kani::internal::kani_forall(0, 2, move |i: usize| identity(enabled) & (i < 2)));
}

#[kani::proof]
fn unused_helper_result() {
    let enabled: bool = kani::any();
    assert!(kani::internal::kani_forall(0, 2, move |i: usize| {
        let _ = identity(enabled);
        i < 2
    }));
}
