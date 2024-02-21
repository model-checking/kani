// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// The original harness takes too long so we introduced a simplified version to run in CI.
// kani-flags: --harness simplified

//! This is a regression test for size_and_align_of_dst computing the
//! size and alignment of a dynamically-sized type like
//! Arc<Mutex<dyn Subscriber>>.
//! We added a simplified version of the original harness from:
//! <https://github.com/model-checking/kani/issues/426>
//! This currently fails due to
//! <https://github.com/model-checking/kani/issues/1781>

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

#[kani::proof]
#[kani::unwind(2)]
fn simplified() {
    let s: Arc<Mutex<dyn Subscriber>> = Arc::new(Mutex::new(DummySubscriber::new()));
    let data = s.lock().unwrap();
    assert!(data.get() == 0);
}

#[kani::proof]
#[kani::unwind(1)]
fn original() {
    let s: Arc<Mutex<dyn Subscriber>> = Arc::new(Mutex::new(DummySubscriber::new()));
    let mut data = s.lock().unwrap();
    data.increment();
    assert!(data.get() == 1);
}
