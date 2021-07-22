// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-verify-fail

// This is handled by the box to box case of unsized pointers

pub trait Trait {
    fn increment(&mut self);
    fn get(&self) -> u32;
}

struct Concrete {
    pub index: u32,
}

impl Concrete {
    fn new() -> Self {
        Concrete { index: 0 }
    }
}

impl Trait for Concrete {
    fn increment(&mut self) {
        self.index = self.index + 1;
    }
    fn get(&self) -> u32 {
        self.index
    }
}

fn main() {
    let mut x: Box<dyn Trait> = Box::new(Concrete::new());
    x.increment();
    assert!(x.get() == 3); // Should be x.get() == 1
}
