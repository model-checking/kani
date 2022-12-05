// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use all_harness::*;

#[cfg(kani)]
mod proofs {
    use super::*;

    #[kani::proof]
    fn with_no_panic() {
        will_rotate_wrap(kani::any(), kani::any());
    }
}
