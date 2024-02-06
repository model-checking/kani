// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

static mut PTR: u32 = 0;

#[kani::modifies(&mut PTR)]
#[kani::ensures(PTR == src)]
unsafe fn modify(src: u32) {
    PTR = src;
}

#[kani::proof]
#[kani::stub_verified(modify)]
fn main() {
    unsafe {
        PTR = kani::any();
        let new = kani::any();
        modify(new);
        kani::assert(new == PTR, "replaced");
    }
}
