// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// @flag --no-memory-splitting --unroll=10
// @expect error

// cbmc-flags: --unwind 5

include!("../../rmc-prelude.rs");

pub fn main() {
    let mut sum = 0;
    let b: u64 = __nondet();
    if b < 5 && b > 1 {
        for i in 0..b {
            sum += i;
        }
        assert!(2 * sum != b * (b - 1));
    }
}
