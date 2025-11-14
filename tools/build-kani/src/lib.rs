// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}
