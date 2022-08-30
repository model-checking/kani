// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test makes sure Kani does not emit "unused variable" warnings for
//! variables that are only used in arguments of assert macros

// Promote "unused variable" warnings to an error so that this test fails if
// Kani's overridden version of the assert macros drops variables used as
// arguments of those macros
#![deny(unused_variables)]

#[kani::proof]
fn check_assert_with_arg() {
    let s = "foo";
    assert!(1 + 1 == 2, "An assertion message that refers to a variable {}", s);
}
