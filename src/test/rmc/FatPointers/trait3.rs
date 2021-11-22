// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Cast a concrete ref to
//   concrete raw pointer
//   trait ref
//   trait raw pointer
// Cast a trait ref to a trait raw pointer

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
    let d = DummySubscriber::new();

    let d1 = &d as *const DummySubscriber;
    assert!(unsafe { d1.as_ref().unwrap().process() } == 1);

    let s = &d as &dyn Subscriber;
    assert!(s.process() == 1);

    let s1 = &d as *const dyn Subscriber;
    assert!(unsafe { s1.as_ref().unwrap().process() } == 1);

    let x = s as *const dyn Subscriber;
    assert!(unsafe { x.as_ref().unwrap().process() } == 1);
}
