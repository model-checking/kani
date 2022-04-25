// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that Kani does not emit colored output that includes escape
// codes when its output is piped/redirected:
// https://github.com/model-checking/kani/issues/844

#[kani::proof]
fn main() {
    assert!(1 + 1 == 2);
    assert!(2 + 2 == 3);
}
