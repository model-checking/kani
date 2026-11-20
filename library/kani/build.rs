// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn main() {
    // Make sure `kani_sysroot` is a recognized config
    println!("cargo::rustc-check-cfg=cfg(kani_sysroot)");
    // `kani` is set by the Kani compiler when verifying user code; recognize it here so that
    // verification-only hook bodies can gate on `cfg(not(kani))` without tripping the
    // `unexpected_cfgs` lint during the library's own build.
    println!("cargo::rustc-check-cfg=cfg(kani)");
}
