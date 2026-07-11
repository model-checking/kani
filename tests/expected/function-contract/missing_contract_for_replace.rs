// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts -Zstubbing

fn no_contract() {}

#[kani::proof]
#[kani::stub_verified(no_contract)]
fn harness() {
    no_contract();
}
