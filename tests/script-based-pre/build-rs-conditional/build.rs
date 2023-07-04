// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Verify that build scripts can check if they are running under `kani`.

fn main() {
    if cfg!(kani) {
        println!("cargo:rustc-env=RUNNING_KANI=Yes");
    } else {
        println!("cargo:rustc-env=RUNNING_KANI=No");
    }
}
