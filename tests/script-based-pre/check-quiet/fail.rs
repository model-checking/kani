// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn always_fails() {
    assert!(false, "this harness must fail");
}
