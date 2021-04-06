// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn call_with_one<F>(mut some_closure: F) -> ()
    where
        F: FnMut(i64, i64) -> (),
{
    some_closure(1, 1);
}

fn __nondet<T>() -> T {
    unimplemented!()
}

pub fn main() {
    let mut num: i32 = __nondet();
    let y = 2;
    if num <= std::i32::MAX - 100 {
        // avoid overflow
        let original_num = num;
        {
            let mut add_num = |x: i64, z: i64| num += y + (x + z) as i32;

            add_num(5, 1);
            call_with_one(&mut add_num);
        }
        assert!(original_num + 12 == num);
    }
}
