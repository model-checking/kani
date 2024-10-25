// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --harness-timeout 5
//
// Check that Kani respects the specified `--harness-timeout` option

#[kani::proof]
fn check_cbmc_timeout() {
    // construct a problem that requires a long time to solve
    let (a1, b1, c1): (u64, u64, u64) = kani::any();
    let (a2, b2, c2): (u64, u64, u64) = kani::any();
    let p1 = a1.saturating_mul(b1).saturating_mul(c1);
    let p2 = a2.saturating_mul(b2).saturating_mul(c2);
    // (a1 == a2 && b1 == b2 && c1 == c2) implies p1 == p2
    assert!(a1 != a2 || b1 != b2 || c1 != c2 || p1 == p2)
}
