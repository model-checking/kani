// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Build script that allows us to build this dependency without bootstrap script.
pub(crate) fn main() {
    // Hard code nightly configuration to build librustdoc.
    println!("cargo:rustc-env=DOC_RUST_LANG_ORG_CHANNEL=nightly");
}
