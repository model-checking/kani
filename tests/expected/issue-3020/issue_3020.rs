// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// compile-flags: -Zmir-opt-level=2
//! Regression test for <https://github.com/model-checking/kani/issues/3020>.
//! At `-Zmir-opt-level=2`, rustc folds `transmute::<u8, bool>(2)` into a `bool` constant whose
//! byte is 2. Kani used to panic on that constant; it should report UB only where it is reached.

#[derive(PartialEq, Clone, Copy)]
struct Wrapper(bool);

#[kani::proof]
fn check_invalid_bool() {
    let b = unsafe { std::mem::transmute::<u8, bool>(2) };
    let _x = b == std::hint::black_box(true);
}

#[kani::proof]
fn check_invalid_bool_in_struct() {
    let w = unsafe { std::mem::transmute::<u8, Wrapper>(2) };
    let _x = w == std::hint::black_box(Wrapper(true));
}

#[kani::proof]
fn check_invalid_bool_not_reached() {
    let reach: bool = kani::any();
    kani::assume(!reach);
    if reach {
        let b = unsafe { std::mem::transmute::<u8, bool>(2) };
        let _x = b == std::hint::black_box(true);
    }
}

#[kani::proof]
fn check_valid_bool() {
    let b = unsafe { std::mem::transmute::<u8, bool>(1) };
    assert!(b == std::hint::black_box(true));
}
