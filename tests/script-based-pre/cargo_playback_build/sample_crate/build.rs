// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Export some variables to the harness

use std::env::var;

fn main() {
    let target = if var("TARGET").unwrap().contains("linux") { "linux" } else { "other" };
    println!(r#"cargo:rustc-cfg=TARGET_OS="{}""#, target);
}
