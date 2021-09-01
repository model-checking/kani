// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-flags: --use-abs --abs-type rmc
include!{"../../rmc-prelude.rs"}

fn main() {
    fn simple_test() {
        let mut vec: Vec<u32> = rmc_vec![1, 2, 3];
        vec.push(3);
        vec.push(4);
        vec.pop();
        assert!(vec.pop() == Some(3));
    }
    
    simple_test();
}
