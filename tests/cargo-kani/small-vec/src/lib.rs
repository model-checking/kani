// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use smallvec::{SmallVec, smallvec};

#[kani::proof]
#[kani::unwind(4)]
pub fn check_vec() {
    // Create small vec with three elements.
    let chars: SmallVec<[char; 3]> = smallvec![kani::any(), kani::any(), kani::any()];
    for c in chars {
        kani::assume(c != char::MAX);
        assert!(c < char::MAX);
    }
}
