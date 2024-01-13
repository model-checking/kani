// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --use-abs --abs-type kani
fn main() {
    fn extend_from_slice_test() {
        let mut vec = kani_vec![1];
        vec.extend_from_slice(&[2, 3, 4]);
        assert!(vec == [1, 2, 3, 4]);
    }

    extend_from_slice_test();
}
