// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// rmc-flags: --no-overflow-checks
// cbmc-flags: --unwind 2

// We use `--no-overflow-checks` in this test to avoid getting
// a verification failure:
// [_RNvCs21hi0yVfW1J_4main14doswitch_chars.overflow.1] line 17 arithmetic overflow on unsigned - in *((unsigned int *)((unsigned char *)&var_7 + 0)) - 1114112: FAILURE
// Tracking issue: https://github.com/model-checking/rmc/issues/307

fn doswitch_int() -> i32 {
    for i in [99].iter() {
        if *i == 99 {
            return 1;
        }
    }
    return 2;
}

fn doswitch_chars() -> i32 {
    for c in "a".chars() {
        if c == 'a' {
            return 1;
        }
    }
    return 2;
}

fn doswitch_bytes() -> i32 {
    for c in "a".bytes() {
        if c == ('a' as u8) {
            return 1;
        }
    }
    return 2;
}

fn main() {
    let v = doswitch_int();
    assert!(v == 1);
    let v = doswitch_chars();
    assert!(v == 1);
    let v = doswitch_bytes();
    assert!(v == 1);
}
