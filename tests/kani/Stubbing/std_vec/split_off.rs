// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --use-abs --abs-type kani
fn main() {
    fn split_off_test() {
        let mut vec = kani_vec![1, 2, 3];
        let vec2 = vec.split_off(1);
        assert!(vec == [1]);
        assert!(vec2 == [2, 3]);

        let mut vec = kani_vec![1, 2, 3];
        let vec2 = vec.split_off(0);
        assert!(vec == []);
        assert!(vec2 == [1, 2, 3]);

        let mut vec = kani_vec![1, 2, 3];
        let vec2 = vec.split_off(3);
        assert!(vec == [1, 2, 3]);
        assert!(vec2 == []);
    }

    split_off_test();
}
