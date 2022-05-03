// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#[derive(Debug, PartialEq)]
pub enum EnumSingle {
    MySingle,
}

#[kani::proof]
fn main() {
    let e = EnumSingle::MySingle;
    assert!(e == EnumSingle::MySingle);
}
