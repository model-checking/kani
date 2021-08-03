// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we can codegen various nesting structures of boxes and
// pointer to closures. Conditions negated for negative test.

// rmc-verify-fail

include!("../../rmc-prelude.rs");

fn main() {
    // Create a nested boxed once-callable closure
    let f: Box<Box<dyn FnOnce(i32)>> =
        Box::new(Box::new(|x| __VERIFIER_expect_fail(x != 1, "wrong int")));
    f(1);

    // Create a pointer to a closure
    let g = |x: f32, y: i32| {
        __VERIFIER_expect_fail(x != 1.0, "wrong float");
        __VERIFIER_expect_fail(y != 2, "wrong int")
    };
    let p: &dyn Fn(f32, i32) = &g;
    p(1.0, 2);

    // Additional level of pointer nesting
    let q: &dyn Fn(f32, i32) = &p;
    q(1.0, 2);

    // Create a boxed pointer to a closure
    let r: Box<&dyn Fn(f32, i32, bool)> = Box::new(&|x: f32, y: i32, z: bool| {
        __VERIFIER_expect_fail(x != 1.0, "wrong float");
        __VERIFIER_expect_fail(y != 2, "wrong int");
        __VERIFIER_expect_fail(!z, "wrong bool");
    });
    r(1.0, 2, true);

    // Another boxed box
    let s: Box<Box<dyn Fn(i32)>> =
        Box::new(Box::new(|x| __VERIFIER_expect_fail(x != 3, "wrong int")));
    s(3);

    // A pointer to the boxed box
    let t: &Box<Box<dyn Fn(i32)>> = &s;
    t(3);
}
