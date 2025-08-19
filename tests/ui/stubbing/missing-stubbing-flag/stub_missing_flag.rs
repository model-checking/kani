// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z function-contracts

// Test that Kani complains when stub is used without the stubbing feature enabled

fn some_function() {}
fn replacement_function() {}

#[kani::stub(some_function, replacement_function)]
#[kani::proof]
fn test_missing_stubbing_flag() {}
