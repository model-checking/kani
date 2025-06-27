// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test vtable function pointer restrictions for dynamic trait objects.

// FIXME until the corresponding CBMC path lands:

// kani-flags: -Z restrict-vtable

struct Sheep {}
struct Cow {}

trait Animal {
    // Instance method signature
    fn noise(&self) -> i32;
}

trait Other {
    // Instance method signature
    fn noise(&self) -> i32;
}

// Implement the `Animal` trait for `Sheep`.
impl Animal for Sheep {
    fn noise(&self) -> i32 {
        1
    }
}

// Implement the `Animal` trait for `Cow`.
impl Animal for Cow {
    fn noise(&self) -> i32 {
        2
    }
}

impl Other for i32 {
    fn noise(&self) -> i32 {
        3
    }
}

// Returns some struct that implements Animal, but we don't know which one at compile time.
fn random_animal(random_number: i64) -> Box<dyn Animal> {
    if random_number < 5 { Box::new(Sheep {}) } else { Box::new(Cow {}) }
}

#[kani::proof]
fn main() {
    let random_number = kani::any();
    let animal = random_animal(random_number);
    let s = animal.noise();
    if random_number < 5 {
        assert!(s == 1);
    } else {
        assert!(s == 2);
    }

    let other = &5 as &dyn Other;
    assert!(other.noise() == 3);
}
