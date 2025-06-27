// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

static mut PTR: u32 = 0;

#[kani::requires(PTR < 100)]
#[kani::modifies(&mut PTR)]
unsafe fn modify() {
    PTR += 1;
}

#[kani::proof]
#[kani::stub_verified(modify)]
fn main() {
    unsafe {
        PTR = kani::any_where(|i| *i < 100);
        let compare = PTR;
        modify();
        kani::assert(compare + 1 == PTR, "not havocked");
    }
}
