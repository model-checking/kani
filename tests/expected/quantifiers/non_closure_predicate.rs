// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers

//! `kani::forall!` and `kani::exists!` always pass a closure as the predicate. Calling the
//! underlying function with a function pointer used to crash the compiler; it is now reported as
//! unsupported, since a pure quantifier body needs the predicate's code.

fn pred(i: usize) -> bool {
    i < 4
}

#[kani::proof]
fn check_fn_pointer_predicate() {
    let _ = kani::internal::kani_forall(0usize, 8usize, pred as fn(usize) -> bool);
}
