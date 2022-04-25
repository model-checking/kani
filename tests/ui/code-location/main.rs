// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --function harness
//
//! This test is to check how file names are displayed in the Kani output.

mod module;

use module::not_empty;

fn same_file(cond: bool) {
    assert!(cond);
}

#[kani::proof]
fn harness() {
    let x = true;
    same_file(x);

    let v = vec![1];
    not_empty(&v);
}

fn main() {}
