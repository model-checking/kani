// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts

struct S(u32);
trait Compute<T> {
    fn compute<U: Copy>(&self, input: T, extra: U) -> u32;
}
impl Compute<u8> for S {
    #[kani::requires(self.0 < 100)]
    #[kani::ensures(|r| *r == self.0)]
    fn compute<U: Copy>(&self, _: u8, _: U) -> u32 {
        self.0
    }
}
impl Compute<u16> for S {
    #[kani::requires(self.0 < 100)]
    #[kani::ensures(|r| *r == self.0 + 1)]
    fn compute<U: Copy>(&self, _: u16, _: U) -> u32 {
        self.0
    }
}

#[kani::proof_for_contract(<S as Compute<u8>>::compute)]
fn generic_contract() {
    let s = S(kani::any());
    let _ = s.compute(1u8, 2u64);
}

#[kani::proof_for_contract(<S as Compute<u16>>::compute)]
#[kani::should_panic]
fn incorrect_generic_contract() {
    let s = S(kani::any());
    let _ = s.compute(1u16, 2u8);
}
