// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers --solver cadical

// Issue #4902: diagnose checked nested predicates before goto-cc aborts.
#[kani::proof]
fn nested_forall() {
    kani::assume(kani::forall!(|i in (0, 4)| kani::forall!(|j in (0, 4)| i + j < 10)));
    assert!(true);
}
