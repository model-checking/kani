// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Examples of cover_or_fail being used outside proof harnesses
// Both of these calls are unreachable from the proof harness,
// yet verification succeeds

fn my_function() {
    kani::cover_or_fail!();
}

#[kani::proof]
fn proof_a() {
    // Since my_function() isn't reachable from the proof harness (i.e., CBMC's entry point),
    // the verification succeeds
    assert!(true);
}

fn my_function_b() {
    if false {
        kani::cover_or_fail!();
    }
}

#[kani::proof]
fn proof_b() {
    // The kani::cover_or_fail call in my_function_b() gets optimized out because it will never be called,
    // so we don't execute the coverage check and verification succeeds
    my_function_b();
    assert!(true);
}
