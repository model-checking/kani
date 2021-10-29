// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-flags: --use-abs --abs-type rmc
fn main() {
    fn insert_test() {
        let mut vec = rmc_vec![1, 2, 3];
        vec.insert(1, 4);
        assert!(vec == [1, 4, 2, 3]);
        vec.insert(4, 5);
        assert!(vec == [1, 4, 2, 3, 5]);
    }

    insert_test();
}
