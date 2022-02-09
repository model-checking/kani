// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Build script that allows us to build this dependency without bootstrap script.
pub fn main() {
    // Hard code nightly configuration to build librustdoc.
    println!("cargo:rustc-env=DOC_RUST_LANG_ORG_CHANNEL=nightly");
}
