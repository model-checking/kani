// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Zconcrete-playback --concrete-playback=print

#[kani::proof]
#[kani::unwind(2)]
fn check_unwind_fail() {
    let mut cnt = 0;
    while kani::any() {
        cnt += 1;
        assert!(cnt < 10);
    }
}
