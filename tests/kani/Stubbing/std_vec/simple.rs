// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --use-abs --abs-type kani
fn main() {
    fn simple_test() {
        let mut vec: Vec<u32> = kani_vec![1, 2, 3];
        vec.push(3);
        vec.push(4);
        vec.pop();
        assert!(vec.pop() == Some(3));
    }

    simple_test();
}
