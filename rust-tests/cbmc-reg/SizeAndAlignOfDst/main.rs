// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// This is a regression test for size_and_align_of_dst computing the
// size and alignment of a dynamically-sized type like
// Arc<Mutex<dyn Subscriber>>.
// This test still fails with a final coercion error for
// DummySubscriber to dyn Subscriber.

use std::mem;
use std::sync::Arc;
use std::sync::Mutex;

pub trait Subscriber {
    fn process(&mut self);
    fn interest_list(&self);
}

struct DummySubscriber {}

impl DummySubscriber {
    fn new() -> Self {
        DummySubscriber {}
    }
}

impl Subscriber for DummySubscriber {
    fn process(&mut self) {}
    fn interest_list(&self) {}
}

fn main() {
    let s: Arc<Mutex<dyn Subscriber>> = Arc::new(Mutex::new(DummySubscriber::new()));
}
