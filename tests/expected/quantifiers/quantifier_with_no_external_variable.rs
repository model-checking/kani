// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// Quantifier with no external variable in the closure

#[kani::proof]
fn test() {
    let quan1 = kani::exists!(|j in (0, 100)| j == 0);
    assert!(quan1);
}
