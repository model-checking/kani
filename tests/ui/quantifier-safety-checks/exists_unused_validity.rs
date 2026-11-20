// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers -Z valid-value-checks --solver cadical

#[inline(never)]
fn identity(value: bool) -> bool {
    value
}
#[kani::proof]
fn exists_unused() {
    assert!(kani::internal::kani_exists(0, 2, |i: usize| {
        let _ = identity(true);
        i > 10
    }));
}
