// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that the print macros do not result in verification failure

fn main() {
    println!("Hello, world!");
    let a = 5;
    let b = "foo";
    let c = false;
    print!("a is {}, b is {}, c is {}\n", a, b, c);
    eprintln!("Bye, world!");
    let s = "bar";
    eprint!("s is {:?}", s);
}
