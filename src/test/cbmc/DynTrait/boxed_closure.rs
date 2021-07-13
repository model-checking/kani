// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//rmc-flags: --no-memory-safety-checks

// We use `--no-memory-safety-checks` in this test to avoid getting
// a verification failure:
// [pointer_dereference.7] invalid function pointer: FAILURE
// Tracking issue: https://github.com/model-checking/rmc/issues/307

// Check that we can codegen a boxed dyn closure

fn main() {
    // Create a boxed once-callable closure
    let f: Box<dyn FnOnce(i32)> = Box::new(|x| assert!(x == 1));

    // Call it
    f(1);
}
