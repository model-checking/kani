// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This ensures a harness can be an associated function. We don't have any oficial restriction
//! today.

struct Dummy {
    c: char,
}

impl Dummy {
    #[kani::proof]
    pub fn new() -> Self {
        Dummy { c: ' ' }
    }
}
