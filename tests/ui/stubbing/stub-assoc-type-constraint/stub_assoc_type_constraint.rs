// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z stubbing

//! Test that stubbing a trait with associated type constraints produces a
//! clear error message.

trait MyIterator {
    type Item;
    fn next(&self) -> Self::Item;
}

struct Counter;
impl MyIterator for Counter {
    type Item = u32;
    fn next(&self) -> u32 {
        1
    }
}

fn mock_next(_: &Counter) -> u32 {
    42
}

#[kani::proof]
#[kani::stub(<Counter as MyIterator<Item = u32>>::next, mock_next)]
fn check_assoc_type_stub() {
    let c = Counter;
    assert_eq!(c.next(), 42);
}
