// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::Dur;

#[kani::proof_for_contract(Dur::half)]
fn check_dur_half() {
    let d = Dur(kani::any());
    let _ = d.half();
}
