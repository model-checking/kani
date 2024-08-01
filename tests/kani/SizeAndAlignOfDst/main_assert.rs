// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This is a regression test for size_and_align_of_dst computing the
//! size and alignment of a dynamically-sized type like
//! Arc<Mutex<dyn Subscriber>>.
//! <https://github.com/model-checking/kani/issues/426>

/// This test fails on macos but not in other platforms.
/// Thus only enable it for platforms where this shall succeed.
#[cfg(not(target_os = "macos"))]
mod not_macos {
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
}
