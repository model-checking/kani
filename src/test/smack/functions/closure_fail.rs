// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// @flag --no-memory-splitting
// @expect verified

fn call_with_one<F>(mut some_closure: F) -> ()
where
    F: FnMut(i32) -> (),
{
    some_closure(1);
}

include!("../../rmc-prelude.rs");

pub fn main() {
    let mut num: i32 = __nondet();
    if num <= std::i32::MAX - 10 {
        let original_num = num;
        {
            let mut add_num = |x: i32| num += x;

            add_num(5);
            call_with_one(&mut add_num);
        }
        assert!(original_num + 6 != num); // Should be old_num + 6
    }
}
