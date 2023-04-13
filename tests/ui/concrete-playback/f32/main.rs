// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --enable-unstable --concrete-playback=print

/// Note: Don't include NaN because there are multiple possible NaN values.
#[kani::proof]
pub fn harness() {
    let f32_1: f32 = kani::any();
    let f32_2: f32 = kani::any();
    let f32_3: f32 = kani::any();
    let f32_4: f32 = kani::any();
    let f32_5: f32 = kani::any();
    let f32_6: f32 = kani::any();
    let f32_7: f32 = kani::any();
    let f32_8: f32 = kani::any();
    assert!(
        !(f32_1 == f32::NEG_INFINITY
            && f32_2 == f32::MIN
            && f32_3 == -101f32
            && (f32_4 == 0f32 && f32_4.signum() < 0.0)
            && f32_5 == f32::MIN_POSITIVE
            && f32_6 == 101f32
            && f32_7 == f32::MAX
            && f32_8 == f32::INFINITY)
    );
}
