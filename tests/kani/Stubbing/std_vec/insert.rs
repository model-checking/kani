// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --use-abs --abs-type kani
fn main() {
    fn insert_test() {
        let mut vec = kani_vec![1, 2, 3];
        vec.insert(1, 4);
        assert!(vec == [1, 4, 2, 3]);
        vec.insert(4, 5);
        assert!(vec == [1, 4, 2, 3, 5]);
    }

    insert_test();
}
