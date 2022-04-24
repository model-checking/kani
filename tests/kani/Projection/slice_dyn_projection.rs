// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --default-unwind 3

//! Check that nested fat pointers work. This used to trigger an issue.
//! The projection should only keep track of the inner most dereferenced
//! element.
//!
//! See: https://github.com/model-checking/kani/issues/378

trait Trait {
    fn id(&self) -> u8;
}

struct Concrete {
    pub id: u8,
}

impl Trait for Concrete {
    fn id(&self) -> u8 {
        self.id
    }
}

#[kani::proof]
fn check_slice_boxed() {
    let boxed_t: &[Box<dyn Trait>] = &[Box::new(Concrete { id: 0 }), Box::new(Concrete { id: 1 })];

    assert_eq!(boxed_t[0].id(), 0);
    assert_eq!(boxed_t[1].id(), 1);
}

#[kani::proof]
#[kani::unwind(3)]
fn check_slice_boxed_iterator() {
    let boxed_t: &[Box<dyn Trait>] = &[Box::new(Concrete { id: 0 }), Box::new(Concrete { id: 1 })];

    // Check iterator
    let mut sum = 0;
    let mut count = 0;
    for obj in boxed_t {
        sum += obj.id();
        count += 1;
    }
    assert_eq!(sum, 1);
    assert_eq!(count, 2);
}
