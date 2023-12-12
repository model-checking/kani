// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Ensure that kani::any can be used with Duration.

use std::time::Duration;

#[kani::proof]
fn check_limits() {
    let any_duration: Duration = kani::any();
    kani::cover!(any_duration == Duration::ZERO, "Zero Duration");
    kani::cover!(any_duration == Duration::MAX, "MAX Duration");
    kani::cover!(any_duration == Duration::from_secs(u64::MAX), "Max Secs");
    kani::cover!(any_duration == Duration::from_millis(u64::MAX), "Max millis");
    kani::cover!(any_duration == Duration::from_micros(u64::MAX), "Max micros");
    kani::cover!(any_duration == Duration::from_nanos(u64::MAX), "Max nanos");

    assert_eq!(any_duration.is_zero(), any_duration == Duration::ZERO, "Is Zero");
}
