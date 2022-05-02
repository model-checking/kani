// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Checks for failures due to unsupported features related to threads.

thread_local! {
    static COND : bool = kani::any();
}

#[kani::proof]
fn main() {
    COND.with(|&b|{
        kani::assume(b);
        assert!(b, "This should fail because we do not support thread local");
    });
}

