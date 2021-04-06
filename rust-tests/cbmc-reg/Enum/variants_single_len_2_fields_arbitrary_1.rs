// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//< https://doc.rust-lang.org/std/convert/enum.Infallible.html (empty enum)

#[derive(Debug)]
pub enum MyInfallible {}

fn foo() -> Result<i64, MyInfallible> {
    Ok(1)
}
fn main() {
    let v = foo().unwrap();
    assert!(v == 1);
}
