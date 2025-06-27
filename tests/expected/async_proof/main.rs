// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// compile-flags: --edition 2018

// Tests that the #[kani::proof] attribute works correctly for async functions

fn main() {}

#[kani::proof(foo)]
async fn test_async_proof_with_options() {}

#[kani::proof]
async fn test_async_proof_on_function_with_arguments(_: ()) {}
