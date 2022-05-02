// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --output-format terse
#[kani::proof]
fn main() {
    assert!(1 + 1 == 3);
}
