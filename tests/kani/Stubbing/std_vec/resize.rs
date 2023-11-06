// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --use-abs --abs-type kani
fn main() {
    fn resize_test() {
        let mut vec = kani_vec![1];
        vec.resize(3, 2);
        assert!(vec == [1, 2, 2]);

        let mut vec = kani_vec![1, 2, 3, 4];
        vec.resize(2, 0);
        assert!(vec == [1, 2]);
    }

    resize_test();
}
