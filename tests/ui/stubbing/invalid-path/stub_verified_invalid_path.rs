// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z function-contracts -Z stubbing

// Test that Kani catches invalid paths in stub_verified attributes

#[kani::stub_verified(nonexistent_function)]
#[kani::proof]
fn test_invalid_path() {}
