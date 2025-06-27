// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that the (e)println with no arguments do not require a trailing semicolon

fn println() {
    println!()
}
fn eprintln() {
    eprintln!()
}

#[kani::proof]
fn main() {
    println();
    eprintln();
}
