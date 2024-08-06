// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// These two are reordered in comparison to `havoc_pass` and we expect the test case to pass still
#[kani::ensures(|result| *dst == src)]
#[kani::modifies(dst)]
fn copy(src: u32, dst: &mut u32) {
    *dst = src;
}

#[kani::proof_for_contract(copy)]
fn copy_harness() {
    copy(kani::any(), &mut kani::any());
}

#[kani::proof]
#[kani::stub_verified(copy)]
fn copy_replace() {
    let src = kani::any();
    let mut dst = kani::any();
    copy(src, &mut dst);
    kani::assert(src == dst, "equality");
}
