// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Testcase for https://github.com/model-checking/kani/issues/558.

enum MyError {
    Val1,
    Val2,
}

fn foo(x: u32) -> Result<(), MyError> {
    if x > 10 { Err(MyError::Val2) } else { Ok(()) }
}

fn bar() -> Result<(), MyError> {
    let x = foo(15)?;

    Ok(x)
}

#[kani::proof]
fn main() {
    bar();
}
