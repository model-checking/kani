// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Example of cover_or_fail being used outside a proof harness with an unreachability failure

fn my_function(x: bool) {
    if x {
        kani::cover_or_fail!();
    }
}

#[kani::proof]
fn proof() {
    // Since my_function() *is* reachable from the proof harness (i.e., CBMC's entry point),
    // but the kani::cover_or_fail!() call *may or may not be* reachable, verification fails
    my_function(false);
}

// Simplified version of the .filter(|_| false).for_each(|_| {} pattern 
// in https://github.com/model-checking/kani/issues/2792
fn test() {
    let my_vec: Vec<u8> = vec![1, 2, 3];
    my_vec.into_iter().filter(|_| false).for_each(|_| {
        kani::cover_or_fail!();
    });
}

#[kani::proof]
fn bolero_use_case() {
    test();
}
