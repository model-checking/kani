// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-verify-fail
// Check that this doesn't trigger a fake loop. See issue #636.
#[no_mangle]
#[kani::proof]
fn main() {
    let x: bool = kani::nondet();
    if x {
        assert!(1 + 1 == 1);
    }
    assert!(1 + 1 == 3, "This one should fail too");
}
