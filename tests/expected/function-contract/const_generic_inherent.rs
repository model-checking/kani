// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

//! A `proof_for_contract` target on a qualified inherent method of a const-generic type
//! (`<Buf<4>>::get`) resolves without the const-generic error: the inherent lookup does not
//! require trait-instance resolution, so the guard in `resolve_in_trait_impl` does not fire.
//! Companion to the trait-target rejection tests (`const_generic_path`/`_omitted`/`_mixed`).

struct Buf<const N: usize> {
    data: [u8; N],
}

impl<const N: usize> Buf<N> {
    #[kani::requires(i < N)]
    fn get(&self, i: usize) -> u8 {
        self.data[i]
    }
}

#[kani::proof_for_contract(<Buf<4>>::get)]
fn check_inherent() {
    let buf = Buf { data: [kani::any(); 4] };
    let i: usize = kani::any();
    kani::assume(i < 4);
    let _ = buf.get(i);
}
