// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --harness harness --enable-unstable --concrete-playback=JustPrint

/// This should generate 2 bools.
/// The first should be 0 because the Option type is Some.
/// The second should be 101, the inner value of the Some.
/// Note: We can't test on a None type yet because the first any::<bool>() could be any non-zero number.
#[kani::proof]
pub fn harness() {
    let option_1: Option<u8> = kani::any();
    assert!(option_1 != Some(101));
}
