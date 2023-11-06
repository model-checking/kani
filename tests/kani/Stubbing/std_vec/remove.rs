// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --use-abs --abs-type kani
fn main() {
    fn remove_test() {
        let mut v = kani_vec![1, 2, 3];
        assert_eq!(v.remove(2), 3);
        assert_eq!(v, [1, 2]);
        assert_eq!(v.remove(1), 2);
        assert_eq!(v.remove(0), 1);

        let mut p = kani_vec![1, 2, 3];
        assert_eq!(p.remove(0), 1);
        assert_eq!(p, [2, 3]);
    }

    remove_test();
}
