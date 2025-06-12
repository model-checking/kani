// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers
/// Quantifier with no external variable in the closure

#[kani::proof]
fn test() {
    let quan = kani::exists!(|j in (0, 100)| j == 0);
    assert!(quan);
}
