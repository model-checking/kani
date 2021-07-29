// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we can codegen various nesting structures of boxes and
// pointer to closures.

// FIXME: several cases fail because we need to "retuple" closures,
// see: https://rust-lang.zulipchat.com/#narrow/stream/182449-t-compiler.2Fhelp/topic/Determine.20untupled.20closure.20args.20from.20Instance.3F

fn main() {
    // Create a nested boxed once-callable closure
    let f: Box<Box<dyn FnOnce(i32)>> = Box::new(Box::new(|x| assert!(x == 1)));
    f(1);

    // Create a pointer to a closure
    let g = |y| assert!(y == 2);
    let p: &dyn Fn(i32) = &g;
    p(2);

    // Additional level of pointer nesting
    let q: &dyn Fn(i32) = &p;
    q(2);

    // Create a boxed pointer to a closure
    let r: Box<&dyn Fn(i32)> = Box::new(&g);
    r(2);

    // Another boxed box
    let s: Box<Box<dyn Fn(i32)>> = Box::new(Box::new(|x| assert!(x == 3)));
    s(3);

    // A pointer to the boxed box
    let t: &Box<Box<dyn Fn(i32)>> = &s;
    t(3);
}
