// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn main() {
    for i in 0..4 {
        debug_assert!(i > 0, "This should fail and stop the execution");
        assert!(i == 0, "This should be unreachable");
    }
}
