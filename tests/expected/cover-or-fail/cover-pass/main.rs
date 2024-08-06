// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// Check that a sorted array may be the same or different than the original

#[kani::proof]
#[kani::unwind(21)]
fn cover_sorted() {
    let arr: [i32; 5] = kani::any();
    let mut sorted = arr.clone();
    sorted.sort();
    kani::cover_or_fail!(sorted == arr);
    kani::cover_or_fail!(sorted != arr);
}
