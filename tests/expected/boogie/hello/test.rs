// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This is an initial test for the Boogie backend that checks that "Hello,
//! Boogie!" is printed when the `--boogie` option is used

// kani-flags: -Zboogie

#[kani::proof]
fn check_boogie_option() {}
