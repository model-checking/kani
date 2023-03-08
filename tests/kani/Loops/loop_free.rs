// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Ensure that Kani identifies that there is not loop in this code.
//! This was related to https://github.com/model-checking/kani/issues/2164
fn loop_free<T: Default>(b: bool, other: T) -> T {
    match b {
        true => T::default(),
        false => other,
    }
}

/// Set the unwind to 1 so this test will fail instead of running forever.
#[kani::proof]
#[kani::unwind(1)]
fn check_no_loop() {
    let b: bool = kani::any();
    let result = loop_free(b, 5);
    assert!(result == 5 || (b && result == 0))
}
