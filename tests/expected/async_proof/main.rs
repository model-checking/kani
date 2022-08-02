// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// compile-flags: --edition 2018

// Tests that the #[kani::async_proof] attribute works correctly

fn main() {}

#[kani::async_proof(foo)]
async fn test_async_proof_with_arguments() {}

#[kani::async_proof]
fn test_async_proof_on_sync_function() {}

#[kani::async_proof]
async fn test_async_proof_on_function_with_inputs(_: ()) {}
