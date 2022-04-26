// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Call a function from crate with global ASM
// Should pass with --ignore-global-asm
#[kani::proof]
fn calls_crate_with_global_asm() {
    let x = 3;
    let y = crate_with_global_asm::eleven();
    assert_eq!(x * y, 33);
}

// Access a static variable from crate with global ASM
// Should pass with --ignore-global-asm
#[kani::proof]
fn reads_static_var_in_crate_with_global_asm() {
    let x = unsafe { crate_with_global_asm::STATIC_VAR };
    assert_eq!(x, 98);
}
