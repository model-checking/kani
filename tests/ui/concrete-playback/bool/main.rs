// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --harness harness --enable-unstable --concrete-playback=JustPrint

/// Note: We can't test a false value yet because any::<bool>() could be any non-zero number.
#[kani::proof]
pub fn harness() {
    let bool_1: bool = kani::any();
    assert!(bool_1 != true);
}
