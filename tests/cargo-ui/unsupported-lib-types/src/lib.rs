// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// The harness bellow will be ignored since they are inside an unsupported crate type.

fn fatal_error(msg: &str) -> ! {
    panic!("[Error]: {}", msg)
}

#[kani::proof]
fn check_error() {
    fatal_error("Oops");
}
