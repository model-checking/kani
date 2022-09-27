// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::env::var;

fn main() {
    // We want to know what target triple we were built with, so "repeat" this info
    // from the build.rs environment into the normal build environment, so we can
    // get it with `env!("TARGET")`
    println!("cargo:rustc-env=TARGET={}", var("TARGET").unwrap());
}
