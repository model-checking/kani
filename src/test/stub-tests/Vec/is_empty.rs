// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-flags: --use-abs --abs-type rmc
fn main() {
    fn is_empty_test() {
        let mut v = Vec::new();
        assert!(v.is_empty());

        v.push(1);
        assert!(!v.is_empty());
    }

    is_empty_test();
}
