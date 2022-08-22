// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks that Kani processes arguments of assert macros and produces
//! an error for invalid arguments (e.g. unknown variable)

#[kani::proof]
fn check_invalid_value_error() {
    assert!(1 + 1 == 2, "An assertion message that references an unknown variable {}", foo);
}
