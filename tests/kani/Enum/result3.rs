// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[derive(Debug, PartialEq)]
pub enum Unit {
    Unit,
}

fn foo(input: &Result<u32, Unit>) -> u32 {
    if let Ok(num) = input { *num } else { 3 }
}

#[kani::proof]
fn main() {
    let input: Result<u32, Unit> = unsafe { kani::any_raw() };
    let x = foo(&input);
    assert!(x == 3 || input != Err(Unit::Unit));
    let input: Result<u32, Unit> = if kani::any() { Ok(0) } else { Err(Unit::Unit) };
    let x = foo(&input);
    assert!(x != 3 || input == Err(Unit::Unit));
    assert!(x != 0 || input == Ok(0));
}
