// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --harness harness --enable-unstable --concrete-playback=Print

#[kani::proof]
pub fn harness() {
    let bool_1: bool = kani::any();
    let bool_2: bool = kani::any();
    assert!(!(!bool_1 && bool_2));
}
