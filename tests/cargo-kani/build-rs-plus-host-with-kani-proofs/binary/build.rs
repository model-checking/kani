// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// From https://github.com/model-checking/kani/issues/3101

use constants::SOME_CONSTANT;

// Having a build script that depends on the constants package
// breaks kani compilation of that package, when compiling the build script.
// I assume it's because the build compile does not set cfg(kani) on the constants package dependency.

fn main() {
    // build.rs changes should trigger rebuild
    println!("cargo:rerun-if-changed=build.rs");

    // Here we have an assertion that gives us additional compile-time checks.
    // In reality, here I read a linker script and assert certain properties in relation to constants defined in the constants package.
    assert_eq!(SOME_CONSTANT, 42);
}
