// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --use-abs --abs-type no-back
fn main() {
    fn append_test() {
        let mut vec = kani_vec![1, 2, 3];
        assert!(vec.len() == 3);
        vec.push(10);
        vec.push(15);
        assert!(vec.len() == 5);
        vec.pop();
        assert!(vec.len() == 4);
        vec.pop();
        vec.pop();
        vec.pop();
        vec.pop();
        vec.pop();
        vec.pop();
        assert!(vec.len() == 0);
        vec.push(15);
        assert!(vec.len() == 1);
    }

    append_test();
}
