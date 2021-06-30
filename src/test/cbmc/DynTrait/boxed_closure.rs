// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we can codegen a boxed dyn closure

fn main() {
    // Create a boxed once-callable closure
    let f: Box<dyn FnOnce()> = Box::new(|x| {
        assert!(x == 1)
    });

    // Call it
    f(1);
}