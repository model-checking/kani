// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Cast a concrete ref to
//   concrete raw pointer
//   trait ref
//   trait raw pointer
// Cast a trait ref to a trait raw pointer

pub trait Subscriber {
    fn process(&mut self);
}

struct DummySubscriber {}

impl DummySubscriber {
    fn new() -> Self {
        DummySubscriber {}
    }
}

impl Subscriber for DummySubscriber {
    fn process(&mut self) {}
}

fn main() {
    let _d = DummySubscriber::new();
    let _d1 = &_d as *const DummySubscriber;

    let _s = &_d as &dyn Subscriber;
    let _s1 = &_d as *const dyn Subscriber;

    let _x = _s as *const dyn Subscriber;
}
