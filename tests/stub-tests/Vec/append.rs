// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --use-abs --abs-type kani
fn main() {
    fn append_test() {
        let mut vec = kani_vec![1, 2, 3];
        let mut vec2 = kani_vec![4, 5, 6];
        vec.append(&mut vec2);
        assert!(vec == [1, 2, 3, 4, 5, 6]);
        assert!(vec2 == []);
    }

    append_test();
}
