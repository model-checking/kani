// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// ANCHOR: code
/// Wrap "too-large" indexes back into a valid range for the array
fn get_wrapped(i: usize, a: &[u32]) -> u32 {
    if a.len() == 0 {
        return 0;
    }
    return a[i % a.len() + 1];
}
// ANCHOR_END: code

// Alternative unsafe return for the above function:
// return unsafe { *a.get_unchecked(i % a.len() + 1) };

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ANCHOR: proptest
    proptest! {
        #[test]
        fn doesnt_crash(i: usize, a: Vec<u32>) {
            get_wrapped(i, &a);
        }
    }
    // ANCHOR_END: proptest
}

// ANCHOR: rmc
#[cfg(rmc)]
#[no_mangle]
fn main() {
    let size: usize = rmc::nondet();
    rmc::assume(size < 4096);
    let index: usize = rmc::nondet();
    let array: Vec<u32> = vec![0; size];
    get_wrapped(index, &array);
}
// ANCHOR_END: rmc
