// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn iadd_test(a: i32, b: i32, correct: i32, wrong: i32) {
    assert!(a + b == correct);
    assert!(a + b == wrong);
}

fn isub_test(a: i32, b: i32, correct: i32, wrong: i32) {
    assert!(a - b == correct);
    assert!(a - b == wrong);
}

fn imul_test(a: i32, b: i32, correct: i32, wrong: i32) {
    assert!(a * b == correct);
    assert!(a * b == wrong);
}

fn idiv_test(a: i32, b: i32, correct: i32, wrong: i32) {
    assert!(a / b == correct);
    assert!(a / b == wrong);
}

fn imod_test(a: i32, b: i32, correct: i32, wrong: i32) {
    assert!(a % b == correct);
    assert!(a % b == wrong);
}

fn ishl_test(a: i32, b: i32, correct: i32, wrong: i32) {
    assert!(a << b == correct);
    assert!(a << b == wrong);
}

fn ishr_test(a: i32, b: i32, correct: i32, wrong: i32) {
    assert!(a >> b == correct);
    assert!(a >> b == wrong);
}

fn ushr_test(a: u32, b: u32, correct: u32, wrong: u32) {
    assert!(a >> b == correct);
    assert!(a >> b == wrong);
}

fn iband_test(a: i32, b: i32, correct: i32, wrong: i32) {
    assert!(a & b == correct);
    assert!(a & b == wrong);
}

fn ibor_test(a: i32, b: i32, correct: i32, wrong: i32) {
    assert!(a | b == correct);
    assert!(a | b == wrong);
}

fn ibxor_test(a: i32, b: i32, correct: i32, wrong: i32) {
    assert!(a ^ b == correct);
    assert!(a ^ b == wrong);
}

fn main() {
    iadd_test(1, 2, 3, 4);
    isub_test(3, 4, -1, 0);
    imul_test(5, 6, 30, 60);
    idiv_test(8, 2, 4, 5);
    idiv_test(9, 2, 4, 5);
    imod_test(9, 3, 0, 1);
    imod_test(10, 3, 1, 2);
    ishl_test(2, 3, 16, 8);
    ishr_test(8, 3, 1, 2);
    ishr_test(-1, 2, -1, 1073741823);
    ushr_test(4294967292, 2, 1073741823, 2);
    iband_test(0, 2389034, 0, 2389034);
    iband_test(3, 10, 2, 3);
    ibor_test(0, 2389034, 2389034, 0);
    ibxor_test(0, 2389034, 2389034, 0);
}