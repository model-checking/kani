// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// This test takes too long with all the std symbols. Use --legacy-linker for now.
// kani-flags: --legacy-linker

//! This is a regression test for size_and_align_of_dst computing the
//! size and alignment of a dynamically-sized type like
//! Arc<Mutex<dyn Subscriber>>.
//! https://github.com/model-checking/kani/issues/426

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
#[kani::unwind(1)]
fn main() {
    let s: Arc<Mutex<dyn Subscriber>> = Arc::new(Mutex::new(DummySubscriber::new()));
    let mut data = s.lock().unwrap();
    data.increment();
    assert!(data.get() == 1);
}
