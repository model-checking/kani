// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers

// See https://github.com/model-checking/kani/issues/4226

#[kani::proof]
#[kani::solver(cvc5)]
fn main() {
    const N: usize = 100;
    let a: [i32; N] = kani::any();
    let i = kani::any();
    kani::assume(i < N - 1);
    kani::assume(kani::forall!(|j in (1, i)| a[j] < 10));
    kani::assume(a[i] < 10);
    assert!(kani::forall!(|j in (1, i+1)| a[j] < 10));
}

#[kani::proof]
fn bounded() {
    const N: usize = 100;
    let a: [i32; N] = kani::any();
    let i = 20;
    kani::assume(i < N - 1);
    kani::assume(kani::forall!(|j in (1, i)| a[j] < 10));
    kani::assume(a[i] < 10);
    assert!(kani::forall!(|j in (1, i+1)| a[j] < 10));
}
