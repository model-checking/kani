// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Ensure Kani can codegen code with a pointer to a function that is never used
//! See <https://github.com/model-checking/kani/issues/3799> for more details.
fn foo<F: Fn()>(_func: &mut F) {}
fn foo_dyn(_func: &mut dyn Fn()) {}

#[kani::proof]
fn check_foo() {
    fn f() {}

    foo(&mut f);
}

#[kani::proof]
fn check_foo_dyn() {
    fn f() {}

    foo_dyn(&mut f);
}

#[kani::proof]
fn check_foo_unused() {
    fn f() {}

    let ptr = &mut f;
}
