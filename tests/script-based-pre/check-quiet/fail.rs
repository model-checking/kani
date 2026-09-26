// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// A harness that always fails, used to check that `--quiet` still reports the failure through the
// exit code (issue #4745).

#[kani::proof]
fn always_fails() {
    assert!(false, "this harness must fail");
}
