// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![deny(warnings)]

fn counted_true(evaluations: &mut u8) -> bool {
    *evaluations += 1;
    *evaluations == 1
}

// Regression for #4874: expression-position cover! calls must still emit coverage properties.
#[kani::proof]
fn cover_expression_contexts() {
    let mut evaluations = 0;

    let _: () = kani::cover!();
    let _: () = kani::cover!(counted_true(&mut evaluations),);
    let _: () = kani::cover!(false, "unsatisfiable cover condition");

    assert!(evaluations == 1);
}
