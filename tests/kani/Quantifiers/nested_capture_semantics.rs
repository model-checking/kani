// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers --solver cadical

#[kani::proof]
fn forall_exists_distinct() {
    assert!(kani::internal::kani_forall(0, 2, move |i: usize| {
        kani::internal::kani_exists(0, 2, move |j: usize| i != j)
    }));
}
#[kani::proof]
#[kani::should_panic]
fn exists_forall_single_value() {
    assert!(kani::internal::kani_exists(0, 2, move |j: usize| {
        kani::internal::kani_forall(0, 2, move |i: usize| i == j)
    }));
}
#[kani::proof]
fn padded_boolean_true() {
    let enabled: bool = kani::any();
    kani::assume(enabled);
    assert!(kani::internal::kani_forall(0, 2, move |i: usize| {
        kani::internal::kani_exists(0, 2, move |j: usize| (i != j) & enabled)
    }));
}
#[kani::proof]
#[kani::should_panic]
fn padded_boolean_false() {
    let enabled: bool = kani::any();
    kani::assume(!enabled);
    assert!(kani::internal::kani_forall(0, 2, move |i: usize| {
        kani::internal::kani_exists(0, 2, move |j: usize| (i != j) & enabled)
    }));
}

#[kani::proof]
fn public_forall_exists_distinct() {
    assert!(kani::forall!(|i in (0, 2)| kani::exists!(|j in (0, 2)| i != j)));
}

#[kani::proof]
#[kani::should_panic]
fn public_exists_forall_single_value() {
    assert!(kani::exists!(|j in (0, 2)| kani::forall!(|i in (0, 2)| i == j)));
}

#[kani::proof]
fn public_boolean_true() {
    let enabled: bool = kani::any();
    kani::assume(enabled);
    assert!(kani::forall!(|i: usize in (0, 2)| {
        kani::exists!(|j: usize in (0, 2)| (i != j) & enabled)
    }));
}

#[kani::proof]
#[kani::should_panic]
fn public_boolean_false() {
    let enabled: bool = kani::any();
    kani::assume(!enabled);
    assert!(kani::forall!(|i: usize in (0, 2)| {
        kani::exists!(|j: usize in (0, 2)| (i != j) & enabled)
    }));
}
