// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --harness harness --enable-unstable --gen-exe-trace

#[kani::proof]
pub fn harness() {
    let char_1: char = kani::any();
    let char_2: char = kani::any();
    assert!(!(char_1 == 'z' && char_2 == char::MAX));
}
