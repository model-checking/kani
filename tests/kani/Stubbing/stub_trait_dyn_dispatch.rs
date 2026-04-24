// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! Test stubbing a trait method called through dynamic dispatch (trait objects).

trait Animal {
    fn sound(&self) -> u32;
}

struct Dog;

impl Animal for Dog {
    fn sound(&self) -> u32 {
        100
    }
}

fn stub_sound(_x: &Dog) -> u32 {
    42
}

fn call_via_dyn(a: &dyn Animal) -> u32 {
    a.sound()
}

#[kani::proof]
#[kani::stub(<Dog as Animal>::sound, stub_sound)]
fn check_stub_dyn_dispatch() {
    let dog = Dog;
    // Direct call — should be stubbed
    assert_eq!(dog.sound(), 42);
    // Call through trait object — stub applies because Kani replaces the
    // function body globally, which is used regardless of dispatch mechanism.
    let result = call_via_dyn(&dog);
    assert_eq!(result, 42);
}
