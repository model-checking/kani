// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

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

fn __nondet<T>() -> T {
    unimplemented!()
}
fn __VERIFIER_assume(cond: bool) {
    unimplemented!()
}

// ANCHOR: rmc
#[cfg(rmc)]
#[no_mangle]
fn main() {
    let a: u32 = __nondet();
    let b: u32 = __nondet();
    simple_addition(a, b);
}
// ANCHOR_END: rmc
