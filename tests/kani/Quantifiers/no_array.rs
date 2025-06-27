// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers

#[kani::proof]
fn forall_assert_harness() {
    let j = kani::any();
    kani::assume(j > 5);
    kani::assert(kani::forall!(|i in (2,5)| i < j ), "");
}

#[kani::proof]
fn forall_assume_harness() {
    let j = kani::any();
    kani::assume(kani::forall!(|i in (2,5)| i < j));
    kani::assert(j > 4, "");
}

fn comp(x: usize, y: usize) -> bool {
    x > y
}

#[kani::proof]
fn forall_function_harness() {
    let j = kani::any();
    kani::assume(j > 5);
    kani::assert(kani::forall!(|i in (2,5)| comp(j, i) ), "");
}

#[kani::proof]
fn exists_assert_harness() {
    let j = kani::any();
    kani::assume(j > 2);
    kani::assert(kani::exists!(|i in (2,5)| i < j ), "");
}

#[kani::proof]
fn exists_assume_harness() {
    let j = kani::any();
    kani::assume(kani::exists!(|i in (2,4)| i == j));
    kani::assert(j == 3 || j == 2, "");
}

#[kani::proof]
fn exists_function_harness() {
    let j = kani::any();
    kani::assume(j > 2);
    kani::assert(kani::exists!(|i in (2,5)| comp(j, i) ), "");
}
