// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// @flag --no-memory-splitting --unroll=4
// @expect verified
// cbmc-flags: --unwind 5

pub fn main() {
    let mut sum = 0;
    let b: u64 = rmc::any();
    if b < 5 && b > 1 {
        for i in 0..b {
            sum += i;
        }
        assert!(2 * sum == b * (b - 1));
    }
}
