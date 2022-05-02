// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::path::Path;

#[kani::proof]
fn main() {
    let path = Path::new("./foo/bar.txt");
}
