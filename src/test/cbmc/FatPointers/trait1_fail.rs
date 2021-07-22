// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-verify-fail

// Cast a concrete ref to a trait ref.

pub trait Subscriber {
    fn process(&self) -> u32;
}

struct DummySubscriber {
    val: u32,
}

impl DummySubscriber {
    fn new() -> Self {
        DummySubscriber { val: 0 }
    }
}

impl Subscriber for DummySubscriber {
    fn process(&self) -> u32 {
        let DummySubscriber { val: v } = self;
        *v + 1
    }
}

fn main() {
    let _d = DummySubscriber::new();
    let _s = &_d as &dyn Subscriber;
    assert!(_s.process() == 3); // Should be _s.process() == 1
}
