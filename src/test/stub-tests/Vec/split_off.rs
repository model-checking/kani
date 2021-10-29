// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-flags: --use-abs --abs-type rmc
fn main() {
    fn split_off_test() {
        let mut vec = rmc_vec![1, 2, 3];
        let vec2 = vec.split_off(1);
        assert!(vec == [1]);
        assert!(vec2 == [2, 3]);

        let mut vec = rmc_vec![1, 2, 3];
        let vec2 = vec.split_off(0);
        assert!(vec == []);
        assert!(vec2 == [1, 2, 3]);

        let mut vec = rmc_vec![1, 2, 3];
        let vec2 = vec.split_off(3);
        assert!(vec == [1, 2, 3]);
        assert!(vec2 == []);
    }

    split_off_test();
}
