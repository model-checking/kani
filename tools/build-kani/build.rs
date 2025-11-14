// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn main() {
    // Collect build environment information
    built::write_built_file().expect("Failed to acquire build-time information");
}
