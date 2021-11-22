// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#[derive(Debug, PartialEq)]
pub enum EnumSingle {
    MySingle,
}

fn main() {
    let e = EnumSingle::MySingle;
    assert!(e == EnumSingle::MySingle);
}
