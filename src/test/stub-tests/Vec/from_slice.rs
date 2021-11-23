// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-flags: --use-abs --abs-type rmc
fn main() {
    fn from_slice_test() {
        assert_eq!(Vec::from(&[1, 2, 3][..]), rmc_vec![1, 2, 3]);
        assert_eq!(Vec::from(&mut [1, 2, 3][..]), rmc_vec![1, 2, 3]);
        assert_eq!(Vec::from([3; 4]), rmc_vec![3, 3, 3, 3]);
    }

    from_slice_test();
}
