// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Binary search with decreases clause.
//! Inspired by CBMC's binary_search example in contracts-decreases documentation.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

fn binary_search(arr: &[i32; 5], target: i32) -> Option<usize> {
    let mut lo: usize = 0;
    let mut hi: usize = arr.len();

    #[kani::loop_invariant(lo <= hi && hi <= arr.len())]
    #[kani::loop_decreases(hi - lo)]
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if arr[mid] == target {
            return Some(mid);
        } else if arr[mid] < target {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }

    None
}

#[kani::proof]
fn binary_search_harness() {
    let arr = [1, 3, 5, 7, 9];
    let result = binary_search(&arr, 5);
    assert!(result == Some(2));
}
