// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --use-abs --abs-type kani
fn main() {
    fn clone_test() {
        let v = kani_vec![1, 2, 3];
        let p = v.clone();

        assert!(p == [1, 2, 3]);
    }

    clone_test();
}
