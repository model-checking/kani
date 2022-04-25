// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
#[kani::unwind(3)]
fn main() {
    let arr = [(1, 2), (2, 2)];
    let result = arr.iter().try_fold((), |acc, &i| Some(()));
    assert_ne!(result, None, "This should succeed");
}
