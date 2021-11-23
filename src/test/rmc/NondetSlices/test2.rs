// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn check(s: &[u8]) {
    let len = s.len();
    assert!(len >= 0 && len < 6);
}

fn main() {
    let slice = rmc::NonDetSlice::<u8, 5>::new();
    check(&slice);
}
