// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn main() {
    // Make sure `kani_sysroot` is a recognized config
    println!("cargo::rustc-check-cfg=cfg(kani_sysroot)");
}
