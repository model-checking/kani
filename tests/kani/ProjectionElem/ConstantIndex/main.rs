// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/enum.ProjectionElem.html
// ConstantIndex
// [-]
// These indices are generated by slice patterns. Easiest to explain by example:

// [X, _, .._, _, _] => { offset: 0, min_length: 4, from_end: false },
// [_, X, .._, _, _] => { offset: 1, min_length: 4, from_end: false },
// [_, _, .._, X, _] => { offset: 2, min_length: 4, from_end: true },
// [_, _, .._, _, X] => { offset: 1, min_length: 4, from_end: true },

fn test1() {
    let a = [1, 2, 3, 4];
    match a {
        [x, _, _, _] => assert!(x == 1),
    }
    match a {
        [_, x, _, _] => assert!(x == 2),
    }
}

fn test2() {
    let a = [1, 2, 3, 4];
    match a {
        [x, .., _] => assert!(x == 1),
    }
    match a {
        [x, ..] => assert!(x == 1),
    }
    match a {
        [.., x, _, _] => assert!(x == 2),
    }
    match a {
        [_, x, ..] => assert!(x == 2),
    }
    match a {
        [_, .., x, _] => assert!(x == 3),
    }
    match a {
        [.., x, _] => assert!(x == 3),
    }
    match a {
        [_, _, x, ..] => assert!(x == 3),
    }
    match a {
        [.., x] => assert!(x == 4),
    }
}

fn test3(a: &[i64]) {
    match a {
        [x, .., _] => assert!(*x == 1),
        _ => assert!(false),
    }
    match a {
        [x, ..] => assert!(*x == 1),
        _ => assert!(false),
    }
    match a {
        [.., x, _, _] => assert!(*x == 2),
        _ => assert!(false),
    }
    match a {
        [_, x, ..] => assert!(*x == 2),
        _ => assert!(false),
    }
    match a {
        [_, .., x, _] => assert!(*x == 3),
        _ => assert!(false),
    }
    match a {
        [.., x, _] => assert!(*x == 3),
        _ => assert!(false),
    }
    match a {
        [_, _, x, ..] => assert!(*x == 3),
        _ => assert!(false),
    }
    match a {
        [.., x] => assert!(*x == 4),
        _ => assert!(false),
    }
}

fn encode_utf8_raw(code: u32, dst: &mut [u8]) -> u8 {
    match (code, &mut dst[..]) {
        (1, [a, ..]) => *a,
        _ => panic!(),
    }
}

fn test4() {
    let code = 1;
    let dst: &mut [u8] = &mut [0u8; 4];
    assert!(encode_utf8_raw(code, dst) == 0);
}

#[kani::proof]
fn main() {
    test1();
    test2();
    test3(&[1, 2, 3, 4]);
    test4();
}
