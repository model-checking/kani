// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-flags: --use-abs --abs-type rmc
fn main() {
    fn into_iter_test() {
        let v = rmc_vec![1, 4, 5];
        let mut iter = v.into_iter();

        assert!(iter.next() == Some(1));
        assert!(iter.next() == Some(4));
        assert!(iter.next() == Some(5));
        assert!(iter.next() == None);

    }

    into_iter_test();
}
