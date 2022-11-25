// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// @flag --no-memory-splitting
// @expect verified
// kani-verify-fail

fn call_with_one<F>(mut some_closure: F) -> ()
where
    F: FnMut(i32) -> (),
{
    some_closure(1);
}

#[kani::proof]
pub fn main() {
    let mut num: i32 = kani::any();
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
