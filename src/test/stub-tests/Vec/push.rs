// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-flags: --use-abs --abs-type rmc
fn main() {
    fn push_test() {
        let mut vec = rmc_vec![1, 2];
        vec.push(3);
        assert!(vec == [1, 2, 3]);
    }

    push_test();
}
