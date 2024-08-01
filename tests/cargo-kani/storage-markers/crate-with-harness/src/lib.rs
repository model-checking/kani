// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that Kani captures the case of a use-after-free issue as
// described in https://github.com/model-checking/kani/issues/3061 even across
// crates. The test calls a function from another crate that has the bug.

#[kani::proof]
pub fn call_fn_with_bug() {
    let _x = crate_with_bug::fn_with_bug();
}
