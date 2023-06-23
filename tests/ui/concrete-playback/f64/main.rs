// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --enable-unstable --concrete-playback=print

/// Note: Don't include NaN because there are multiple possible NaN values.
#[kani::proof]
pub fn harness() {
    let f64_1: f64 = kani::any();
    let f64_2: f64 = kani::any();
    let f64_3: f64 = kani::any();
    let f64_4: f64 = kani::any();
    let f64_5: f64 = kani::any();
    let f64_6: f64 = kani::any();
    let f64_7: f64 = kani::any();
    let f64_8: f64 = kani::any();
    assert!(
        !(f64_1 == f64::NEG_INFINITY
            && f64_2 == f64::MIN
            && f64_3 == -101f64
            && (f64_4 == 0f64 && f64_4.signum() < 0.0)
            && f64_5 == f64::MIN_POSITIVE
            && f64_6 == 101f64
            && f64_7 == f64::MAX
            && f64_8 == f64::INFINITY)
    );
}
