// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Kani should error out if run without --ignore-global-asm even if the crate
// with global ASM is not called
#[kani::proof]
fn doesnt_call_crate_with_global_asm() {
    let x = 3;
    let y = 11;
    assert_eq!(x * y, 33);
}
