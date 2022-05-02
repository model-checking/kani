// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! test that we implement closures correctly

fn closure_with_empty_args() {
    let bytes = vec![0];
    let b = bytes.get(0).ok_or_else(|| ()).unwrap();
    assert!(*b == 0);
}

fn closure_with_1_arg() {
    let b = Some(3);
    let r = b.map(|x| x + 1);
    assert!(r == Some(4));
}

fn takes_unit_args<F: FnOnce(i32, (), i32) -> i32>(f: F) -> i32 {
    f(1, (), 3)
}

fn test_unit_args() {
    let r = takes_unit_args(|a, _b, c| a + c);
    assert!(r == 4);
}

fn takes_three_args<F: FnOnce(i32, i32, i32) -> i32>(f: F) -> i32 {
    f(1, 2, 3)
}

fn test_three_args() {
    let r = takes_three_args(|a, b, c| a + b + c);
    assert!(r == 6);
}

fn test_env() {
    let x = 3;
    let r = takes_three_args(|a, b, c| a + b + c + x);
    assert!(r == 9);
}

#[kani::proof]
fn main() {
    closure_with_empty_args();
    closure_with_1_arg();
    test_three_args();
    test_unit_args();
    test_env();
}
