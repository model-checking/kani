// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// rmc-flags: --no-unwinding-checks
// cbmc-flags: --unwind 10

// We use `--no-unwinding-checks` in this test to avoid getting
// a verification failure (the loop being unwound depends on
// a nondet. variable)

include!("../../rmc-prelude.rs");

fn main() {
    let x: u32 = __nondet();
    let pi = 3.14159265359;

    let x_iters = leibniz_pi(x);
    let x_plus_1_iters = leibniz_pi(x + 1);

    // prove approximation is within 0.1 of pi, given enough iterations
    if x >= 9 {
        let diff = abs_f32(x_iters - pi);
        assert!(diff < 0.1);
    }

    // prove that each iteration improves from the previous
    assert!(abs_f32(x_plus_1_iters - pi) < abs_f32(x_iters - pi));
}
fn leibniz_pi(num_iterations: u32) -> f32 {
    let mut i = 0;
    let mut denominator = 1.0;
    let mut sign = 1.0;
    let mut running_total = 1.0;

    while i < num_iterations {
        // prepare current step
        denominator += 2.0;
        sign *= -1.0;
        i += 1;
        // add current step
        running_total += sign / denominator;
    }

    // above formula calculates pi / 4;
    running_total *= 4.0;

    return running_total;
}
fn abs_f32(value: f32) -> f32 {
    if value >= 0.0 {
        return value;
    } else {
        return -value;
    }
}
