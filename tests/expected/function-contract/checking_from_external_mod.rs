// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

#[kani::ensures((result == x) | (result == y))]
fn max(x: u32, y: u32) -> u32 {
    if x > y { x } else { y }
}

mod harnesses {
    #[kani::proof_for_contract(super::max)]
    fn main() {
        let _ = Box::new(9_usize);
        super::max(7, 6);
    }
}

