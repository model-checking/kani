// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-verify-fail

// ANCHOR: code
fn simple_addition(a: u32, b: u32) -> u32 {
    return a + b;
}
// ANCHOR_END: code

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ANCHOR: proptest
    proptest! {
        #[test]
        fn doesnt_crash(a: u32, b: u32) {
            simple_addition(a, b);
        }
    }
    // ANCHOR_END: proptest
}

// ANCHOR: rmc
#[cfg(rmc)]
#[no_mangle]
fn main() {
    let a: u32 = rmc::nondet();
    let b: u32 = rmc::nondet();
    simple_addition(a, b);
}
// ANCHOR_END: rmc
