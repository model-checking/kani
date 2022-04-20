// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that calling a function from a crate that has global ASM results in verification failure
#[kani::proof]
fn calls_crate_with_global_asm() {
    let x = 3;
    let y = crate_with_global_asm::eleven();
    assert_eq!(3 * y, 33);
}

// Checks that verification passes if the crate with global ASM is not called
// (even though it's still a dependent crate)
#[kani::proof]
fn doesnt_call_crate_with_global_asm() {
    let x = 3;
    let y = 11;
    assert_eq!(x * y, 33);
}
