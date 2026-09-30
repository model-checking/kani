// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z uninit-checks --only-codegen

//! Regression test: the delayed-UB instrumentation walks the ancestors of places in the
//! points-to graph level by level. The graph can contain cycles (here, `Vec::push`'s
//! allocation path makes `v` appear to point to itself), and the walk used to loop forever,
//! hanging the compiler. Check that instrumentation terminates.

#[kani::proof]
fn check_vec_push() {
    let mut v: Vec<u16> = Vec::with_capacity(2);
    v.push(1);
    assert_eq!(v[0], 1);
}
