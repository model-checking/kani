// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use kani::Invariant;
use smallvec::{smallvec, SmallVec};

#[kani::proof]
pub fn check_vec() {
    // Create small vec with three elements.
    let chars: SmallVec<[char; 3]> = smallvec![kani::any(), kani::any(), kani::any()];
    for c in chars {
        assert!(c.is_valid());
    }
}
