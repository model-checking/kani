// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-flags: --use-abs --abs-type rmc
fn main() {
    fn append_test() {
        let mut vec = rmc_vec![1, 2, 3];
        let mut vec2 = rmc_vec![4, 5, 6];
        vec.append(&mut vec2);
        assert!(vec  == [1, 2, 3, 4, 5, 6]);
        assert!(vec2 == []);
    }

    append_test();
}
