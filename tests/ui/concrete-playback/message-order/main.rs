// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --concrete-playback=print --harness dummy -Z concrete-playback

#[kani::proof]
fn dummy() {
    kani::cover!(kani::any::<u32>() != 10);
}
