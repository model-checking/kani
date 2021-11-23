// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-flags: --use-abs --abs-type rmc
fn main() {
    fn clone_test() {
        let v = rmc_vec![1, 2, 3];
        let p = v.clone();

        assert!(p == [1, 2, 3]);
    }

    clone_test();
}
