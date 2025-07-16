// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers

/// Example of code using multiple quantifiers

#[kani::proof]
fn main() {
    let quan1 = kani::forall!(|i in (4, 100)| i < 1000);
    let quan2 = kani::forall!(|i in (2, 6)| i < 7 );
    let quan3 = kani::exists!(|i in (0, 10)| i == 8);
    let quan4 = kani::exists!(|i in (0, 9)| i % 2 == 0);
    assert!(quan1);
    assert!(quan2);
    assert!(quan3);
    assert!(quan4);
}
