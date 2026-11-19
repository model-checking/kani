// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Regression for https://github.com/model-checking/kani/issues/4874:
//! No-message assertions must expand to expressions without trailing semicolons.

#![deny(warnings)]

const _: () = assert!(true);
const _: () = const { assert!(true) };

fn require_unit(_: ()) {}

#[kani::proof]
fn expression_contexts() {
    assert!(true);
    let () = assert!(true);
    require_unit(assert!(true,));
    let () = { assert!(true) };
    match kani::any::<bool>() {
        true => assert!(true),
        false => assert!(true,),
    }
}

#[kani::proof]
fn condition_is_evaluated_once() {
    let mut evaluations = 0u8;
    assert!({
        evaluations += 1;
        true
    });
    assert_eq!(evaluations, 1);
}

#[kani::proof]
#[kani::should_panic]
fn false_assertion_still_fails() {
    assert!(false);
}
