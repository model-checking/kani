// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-flags: --use-abs --abs-type rmc
fn main() {
    fn shrink_to_test() {
        let mut vec = Vec::with_capacity(10);
        vec.extend([1, 2, 3]);
        assert!(vec.capacity() == 10);
        vec.shrink_to(4);
        assert!(vec.capacity() >= 4);
        vec.shrink_to(0);
        rmc::expect_fail(vec.capacity() >= 3, "Capacity shrinked to 0");
    }

    shrink_to_test()
}
