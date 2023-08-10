// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

extern crate kani;

#[derive(Copy, Clone, PartialEq, Eq, kani::Arbitrary)]
struct WrappedInt(u32);

impl WrappedInt {
    #[kani::ensures((result == self) | (result == y))]
    fn max(self, y: WrappedInt) -> WrappedInt {
        Self(if self.0 > y.0 { self.0 } else { y.0 })
    }
}

#[kani::proof_for_contract(WrappedInt::max)]
fn max_harness() {
    let _ = Box::new(9_usize);
    WrappedInt(7).max(WrappedInt(6));
}
