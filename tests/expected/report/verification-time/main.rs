// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that the time spent in CBMC is reported as "Verification time: <>s"

fn is_sorted(s: &[i32]) -> bool {
    for i in 0..s.len() - 1 {
        if !(s[i] <= s[i + 1]) {
            return false;
        }
    }
    true
}

#[kani::proof]
#[kani::unwind(6)]
fn check_sorted() {
    let mut arr: [i32; 5] = kani::any();
    arr.sort();
    assert!(is_sorted(&arr));
}
