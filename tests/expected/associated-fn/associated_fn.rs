// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --enable-unstable --function new
//! This ensures our public functions reachability module works for associated functions.

struct Dummy {
    c: char,
}

impl Dummy {
    #[no_mangle]
    pub fn new() -> Self {
        Dummy { c: ' ' }
    }
}
