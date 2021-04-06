// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#[derive(Debug, PartialEq)]
pub enum Empty {}

pub fn main() {
    let res: Result<u32, Empty> = Ok(0);
    if let Ok(num) = res {
        num + 1;
    } else {
        3;
    }
}
