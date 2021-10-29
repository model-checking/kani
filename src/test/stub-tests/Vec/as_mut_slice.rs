// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-flags: --use-abs --abs-type rmc
fn main() {
    fn as_mut_slice_test() {
        let mut buffer = rmc_vec![1, 2, 3];
        buffer.as_mut_slice().reverse();
        assert!(buffer == [3, 2, 1]);
    }

    as_mut_slice_test();
}
