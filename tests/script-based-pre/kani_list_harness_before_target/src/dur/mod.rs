// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// A harness module declared before the function it verifies.
#[cfg(kani)]
mod proofs;

pub struct Dur(pub u8);

impl Dur {
    #[cfg_attr(kani, kani::ensures(|r: &u8| *r <= 127))]
    pub fn half(&self) -> u8 {
        self.0 / 2
    }
}

// A second harness declared after the function, so both orders are listed.
#[cfg(kani)]
#[kani::proof_for_contract(Dur::half)]
fn check_dur_half_after() {
    let d = Dur(kani::any());
    let _ = d.half();
}
