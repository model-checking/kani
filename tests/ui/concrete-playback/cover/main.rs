// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --enable-unstable --concrete-playback=print

#[kani::proof]
pub fn harness() {
    let x: i32 = kani::any();
    kani::cover!(x != 0 && x / 2 == 0);
}
