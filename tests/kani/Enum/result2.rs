// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#[derive(Debug, PartialEq)]
pub enum Empty {}

#[kani::proof]
fn main() {
    let res: Result<Empty, u32> = Err(0);
    if let Err(num) = res {
        num + 1;
    } else {
        3;
    }
}
