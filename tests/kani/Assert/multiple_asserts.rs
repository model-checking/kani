// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-verify-fail
// Check that this doesn't trigger a fake loop. See issue #636.
#[kani::proof]
fn main() {
    let x: bool = kani::any();
    if x {
        assert!(1 + 1 == 1);
    }
    assert!(1 + 1 == 3, "This one should fail too");
}
