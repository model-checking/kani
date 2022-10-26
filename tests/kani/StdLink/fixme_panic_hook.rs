// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test was created to cover panic hook handling by Kani.
//! Tracking issue: <https://github.com/model-checking/kani/issues/208>
use std::panic;

#[kani::proof]
#[kani::unwind(2)]
fn custom_hook() {
    panic::set_hook(Box::new(|_| {
        assert!(false);
    }));

    let _ = panic::take_hook();

    panic!("Normal panic");
}
