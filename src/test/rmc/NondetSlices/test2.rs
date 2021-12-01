// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Test the RMC library's API for creating a non-det slice

fn check(s: &[u8]) {
    let len = s.len();
    assert!(len >= 0 && len < 6, "Expected slice length to be between 0 and 5. Got {}.", len);
}

fn main() {
    // returns a slice of length between 0 and 5 with non-det content
    let slice: rmc::slice::NonDetSlice<u8, 5> = rmc::slice::nondet_slice();
    check(&slice);
}
