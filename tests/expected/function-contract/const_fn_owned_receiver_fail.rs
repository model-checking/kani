// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

//! A deliberately violated ensures on a `const fn` with an owned `Drop`
//! receiver must still fail: the receiver-move in the replace body must not
//! weaken checking. `Owned` wraps a `u32` (no `Box` deref in the ensures) so
//! the only failing check is the false postcondition itself.

struct Owned(u32);

impl Drop for Owned {
    fn drop(&mut self) {}
}

impl Owned {
    #[kani::ensures(|result: &Owned| result.0 == old(self.0) + 1)]
    const fn identity(self) -> Owned {
        self
    }
}

#[kani::proof_for_contract(Owned::identity)]
fn harness() {
    let v: u32 = kani::any_where(|v: &u32| *v < 100);
    let _ = Owned(v).identity();
}
