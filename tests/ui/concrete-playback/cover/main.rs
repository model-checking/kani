// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Zconcrete-playback --concrete-playback=print

#[kani::proof]
pub fn harness() {
    let x: u32 = kani::any();
    kani::cover!(x != 0 && x / 2 == 0);
}
