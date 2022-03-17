// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

use std::fs;
use std::path::PathBuf;
#[kani::proof]
fn main() {
    let buf = PathBuf::new();
    let _x = fs::remove_dir_all(buf);
}
