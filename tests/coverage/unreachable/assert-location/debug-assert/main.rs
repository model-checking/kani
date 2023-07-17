// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check the location of the assert statement when using debug_assert!
// This currently requires the old format since the new format doesn't include
// line numbers https://github.com/model-checking/kani/issues/918
// Also disable reachability checks because they add annotations to each
// assert's description which would be visible with the old output format
#[kani::proof]
fn main() {
    for i in 0..4 {
        debug_assert!(i > 0, "This should fail and stop the execution");
        assert!(i == 0, "This should be unreachable");
    }
}
