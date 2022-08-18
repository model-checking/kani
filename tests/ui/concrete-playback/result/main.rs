// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --harness harness --enable-unstable --concrete-playback=JustPrint

/// This should generate 2 bools.
/// The first should be 0 because the Result type is Ok.
/// The second should be 101, the inner value of the Ok.
/// Note: We can't test an Err type yet because the first any::<bool>() could be any non-zero number.
#[kani::proof]
pub fn harness() {
    let result_1: Result<u8, u8> = kani::any();
    assert!(result_1 != Ok(101));
}
