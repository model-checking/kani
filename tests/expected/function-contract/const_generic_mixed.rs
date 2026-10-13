// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

//! A `proof_for_contract` target on a type with both a type and a const parameter, with the
//! const written (`<Pair<u8, 4> as Get>::get`), is refused with a clean error. The free type
//! parameter means this path does not ICE, but the explicit error is clearer than a
//! downstream resolution failure.

struct Pair<T, const N: usize> {
    first: T,
    rest: [u8; N],
}

trait Get {
    fn get(&self, i: usize) -> u8;
}

impl<T, const N: usize> Get for Pair<T, N> {
    #[kani::requires(i < N)]
    fn get(&self, i: usize) -> u8 {
        self.rest[i]
    }
}

#[kani::proof_for_contract(<Pair<u8, 4> as Get>::get)]
fn check_mixed() {
    let pair = Pair { first: 0u8, rest: [kani::any(); 4] };
    let i: usize = kani::any();
    kani::assume(i < 4);
    let _ = pair.get(i);
}
