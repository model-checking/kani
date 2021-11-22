// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
/// This test touches a surprising amount of MIR, thanks to the `assert_eq!`, which translates into something like
/// if(x != y) { String msg = format_error_message(<x as debug>::fmt(x), <y as debug>::fmt(y)); panic!(msg)} else {}
/// This leads us to the land of foreign types, ReifyFnPointer, and transmute.
/// The "C" output from RMC is about 1KLOC, vs 80LOC for the same version with straight `assert!`.
///     https://github.com/model-checking/rmc/issues/14
/// The assertion message printed to the user on success is uninformative:
///     "library/std/src/macros.rs line 17 a panicking function core::panicking::panic_fmt is invoked: SUCCESS"
///     https://github.com/model-checking/rmc/issues/13

fn main() {
    let x = 1;
    let y = 2;
    assert_eq!(x + 1, y);
    assert_eq!(x, y); //Expected failure
    assert_ne!(x, y);
}
