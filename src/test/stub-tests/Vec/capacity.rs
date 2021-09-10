// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-flags: --use-abs --abs-type rmc
include!{"../../rmc-prelude.rs"}

fn main() {
    fn capacity_test() {
        let mut vec = Vec::with_capacity(10);

        // The vector contains no items, even though it has capacity for more
        assert_eq!(vec.len(), 0);
        assert_eq!(vec.capacity(), 10);

        // These are all done without reallocating...
        for i in 0..10 {
            vec.push(i);
        }

        assert_eq!(vec.len(), 10);
        assert_eq!(vec.capacity(), 10);

        // ...but this may make the vector reallocate
        vec.push(11);
        assert_eq!(vec.len(), 11);
        assert!(vec.capacity() >= 11);
    }

    capacity_test()
}
