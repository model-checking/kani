// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// This is a regression test for size_and_align_of_dst computing the
// size and alignment of a dynamically-sized type like
// Arc<Mutex<dyn Subscriber>>.
// This test still fails with a final coercion error for
// DummySubscriber to dyn Subscriber.

#![feature(layout_for_ptr)]
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
    let v = unsafe { mem::size_of_val_raw(&5i32) };
    assert!(v == 4);

    let x: [u8; 13] = [0; 13];
    let y: &[u8] = &x;
    let v = unsafe { mem::size_of_val_raw(y) };
    assert!(v == 13);

    let s: Arc<Mutex<dyn Subscriber>> = Arc::new(Mutex::new(DummySubscriber::new()));
}
