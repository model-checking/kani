// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// From https://github.com/model-checking/kani/issues/3101

use constants::SOME_CONSTANT;

fn main() {
    // build.rs changes should trigger rebuild
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(not(kani_host))]
    assert_eq!(constants::SOME_CONSTANT, 0);
    #[cfg(kani_host)]
    assert_eq!(constants::SOME_CONSTANT, 2);
}
