// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers --solver cadical

#[kani::proof]
fn nested_overflow_must_not_disappear() {
    assert!(kani::forall!(|i in (0, 2)| {
        kani::exists!(|j in (0, 2)| {
            let _ = i + usize::MAX;
            i != j
        })
    }));
}
