// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

//! A `proof_for_contract` target on a const-only type whose const argument is omitted
//! (`<Buf as Get>::get`) still leaves a free const parameter. Kani must report a clean
//! unsupported-path error, not panic with an internal compiler error.

struct Buf<const N: usize> {
    data: [u8; N],
}

trait Get {
    fn get(&self, i: usize) -> u8;
}

impl<const N: usize> Get for Buf<N> {
    #[kani::requires(i < N)]
    fn get(&self, i: usize) -> u8 {
        self.data[i]
    }
}

#[kani::proof_for_contract(<Buf as Get>::get)]
fn check_omitted() {
    let buf = Buf { data: [kani::any(); 4] };
    let i: usize = kani::any();
    kani::assume(i < 4);
    let _ = buf.get(i);
}
