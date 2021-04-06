// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
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
