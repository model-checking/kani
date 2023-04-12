// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test is to check how Kani handle enums where only one variant is valid.
#![feature(never_type)]

enum MyResult<Y, N, M> {
    Yes(Y),
    No(N),
    Maybe(M),
}

fn change_maybe<Y, N, M, O>(orig: MyResult<Y, N, M>, val: O) -> MyResult<Y, N, O> {
    match orig {
        MyResult::Yes(y) => MyResult::Yes(y),
        MyResult::No(n) => MyResult::No(n),
        MyResult::Maybe(m) => MyResult::Maybe(val),
    }
}

fn check() -> Result<u32, !> {
    let val = Result::<u32, !>::Ok(10)?;
    Ok(val)
}

fn checkErr() -> Result<!, u32> {
    let val = Result::<!, u32>::Err(10)?;
}

fn checkMaybe() -> MyResult<!, !, u8> {
    change_maybe(MyResult::<!, !, u32>::Maybe(10), 0)
}

#[kani::proof]
pub fn harness_residual() {
    let _ = checkMaybe();
    let _ = checkErr();
    let _ = check();
}
