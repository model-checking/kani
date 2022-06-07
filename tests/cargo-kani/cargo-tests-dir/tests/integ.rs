// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use cargo_tests_dir::ONE; // trigger dependency resolution

#[kani::proof]
fn check_import() {
    assert!(ONE == 1);
}
