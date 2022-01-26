// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --use-abs --abs-type kani
fn main() {
    fn truncate_test() {
        let mut vec = kani_vec![1, 2, 3];
        vec.truncate(8);
        assert_eq!(vec, [1, 2, 3]);
    }

    truncate_test();
}
