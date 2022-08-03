// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test is meant for checking that the "Verification time:" line (which
// reports the time spent in CBMC) is printed in the output

fn is_sorted(s: &[i32]) -> bool {
    for i in 0..s.len() - 1 {
        if !(s[i] <= s[i + 1]) {
            return false;
        }
    }
    true
}

#[kani::proof]
#[kani::unwind(7)]
fn check_sorted() {
    let mut arr: [i32; 5] = kani::any();
    arr.sort();
    assert!(is_sorted(&arr));
}
