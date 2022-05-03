// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks static variables declared inside methods are still unique.

// There should be no name collision.
static counter: i8 = 0;

fn new_id() -> i8 {
    static mut counter: i8 = 0;
    unsafe {
        counter += 1;
        counter
    }
}

#[kani::proof]
fn main() {
    let id_1 = new_id();
    let id_2 = new_id();
    assert!(id_1 == 1);
    assert!(id_2 == 2);
    assert!(counter == 0);
}
