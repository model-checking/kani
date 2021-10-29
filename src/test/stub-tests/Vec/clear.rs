// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-flags: --use-abs --abs-type rmc
fn main() {
    fn clear_test() {
        let mut v = rmc_vec![1, 2, 3];

        v.clear();

        assert!(v.is_empty());
    }

    clear_test();
}
