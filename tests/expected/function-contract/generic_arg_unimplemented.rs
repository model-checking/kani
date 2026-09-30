// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// A proof_for_contract on an instantiation that is NOT implemented (`Generic<Wrap<u16>>`,
// where only `Generic<Wrap<u8>>` exists) must fail to resolve. Guards that instantiating
// the path's generic arguments (kani#1997) does not over-resolve to the wrong impl.

struct S(u32);
struct Wrap<T>(T);

trait Generic<T> {
    fn generic(&self) -> u32;
}

impl Generic<Wrap<u8>> for S {
    #[kani::requires(self.0 < 100)]
    #[kani::ensures(|r| *r == self.0)]
    fn generic(&self) -> u32 {
        self.0
    }
}

#[cfg(kani)]
mod verify {
    use super::*;

    #[kani::proof_for_contract(<S as Generic<Wrap<u16>>>::generic)]
    fn check_unimplemented() {
        let s = S(kani::any());
        let _ = Generic::<Wrap<u8>>::generic(&s);
    }
}
