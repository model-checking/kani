// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// This is a regression test for size_and_align_of_dst computing the
// size and alignment of a dynamically-sized type like
// Arc<Mutex<dyn Subscriber>>.
// This test still fails with a final coercion error for
// DummySubscriber to dyn Subscriber.

use std::sync::Arc;
use std::sync::Mutex;

pub trait Subscriber {
    fn process(&self);
    fn increment(&mut self);
    fn get(&self) -> u32;
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
    fn process(&self) {}
    fn increment(&mut self) {
        self.val = self.val + 1;
    }
    fn get(&self) -> u32 {
        self.val
    }
}

fn main() {
    let s: Arc<Mutex<dyn Subscriber>> = Arc::new(Mutex::new(DummySubscriber::new()));
    let mut data = s.lock().unwrap();
    data.increment();
    assert!(data.get() == 1);
}
