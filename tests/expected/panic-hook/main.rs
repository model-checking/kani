// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that panic::set_hook and panic::take_hook work as expected

use std::panic;

fn main() {
    panic::set_hook(Box::new(|panic_info| {
        if let Some(location) = panic_info.location() {
            // This assertion should be reachable, but it doesn't appear in the
            // results:
            // https://github.com/model-checking/kani/issues/946
            assert!(location.line() == 14);
        }
    }));

    if kani::any() {
        panic!("Panic with custom hook");
    }

    let _ = panic::take_hook();

    panic!("Normal panic");
}
