// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Zconcrete-playback --concrete-playback=print

#[kani::proof]
pub fn harness() {
    let isize_1: isize = kani::any();
    let isize_2: isize = kani::any();
    let isize_3: isize = kani::any();
    let isize_4: isize = kani::any();
    let isize_5: isize = kani::any();
    assert!(
        !(isize_1 == isize::MIN
            && isize_2 == -101
            && isize_3 == 0
            && isize_4 == 101
            && isize_5 == isize::MAX)
    );
}
