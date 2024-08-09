// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// Check that a failed unwinding assertion would lead to reporting a cover
/// property as UNDETERMINED if it's not satisfiable with the given unwind bound

#[kani::proof]
#[kani::unwind(10)]
fn cover_undetermined() {
    let x = [1; 10];
    let mut sum = 0;
    for i in x {
        sum += i;
    }
    kani::cover_or_fail!(sum == 10);
}
