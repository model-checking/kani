// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --use-abs --abs-type kani
fn main() {
    fn swap_remove_test() {
        let mut v = kani_vec![1, 2, 3, 4];

        assert_eq!(v.swap_remove(1), 2);
        assert_eq!(v, [1, 4, 3]);

        assert_eq!(v.swap_remove(0), 1);
        assert_eq!(v, [3, 4]);
    }

    swap_remove_test();
}
