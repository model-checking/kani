// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#[kani::proof]
fn main() {
    assert!(1 == 1);
}

#[test]
/// Purpose of this is to check if Kani can comple this code in
/// verification mode when `kani::concrete_playback_run` is in the
/// code,
fn _playback_type_checks() {
    kani::concrete_playback_run(vec![], test_sum);
}
