// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// compile-flags: --edition 2018
// kani-flags: --no-unwinding-checks

// This test is to check that the `schedule` argument requires an unstable flag.

#[kani::proof(schedule = kani::RoundRobin::default())]
async fn test() {
    assert!(true);
}
