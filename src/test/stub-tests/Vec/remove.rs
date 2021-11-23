// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-flags: --use-abs --abs-type rmc
fn main() {
    fn remove_test() {
        let mut v = rmc_vec![1, 2, 3];
        assert_eq!(v.remove(2), 3);
        assert_eq!(v, [1, 2]);
        assert_eq!(v.remove(1), 2);
        assert_eq!(v.remove(0), 1);

        let mut p = rmc_vec![1, 2, 3];
        assert_eq!(p.remove(0), 1);
        assert_eq!(p, [2, 3]);
    }

    remove_test();
}
