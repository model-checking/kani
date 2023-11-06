// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --use-abs --abs-type kani
fn main() {
    fn from_slice_test() {
        assert_eq!(Vec::from(&[1, 2, 3][..]), kani_vec![1, 2, 3]);
        assert_eq!(Vec::from(&mut [1, 2, 3][..]), kani_vec![1, 2, 3]);
        assert_eq!(Vec::from([3; 4]), kani_vec![3, 3, 3, 3]);
    }

    from_slice_test();
}
