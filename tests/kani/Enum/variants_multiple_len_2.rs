// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#[derive(Debug)]
pub enum EnumMultiple {
    Multiple1,
    Multiple2,
}

#[kani::proof]
fn main() {
    let e = EnumMultiple::Multiple1;
}
