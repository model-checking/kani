// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --use-abs --abs-type kani
fn main() {
    fn push_test() {
        let mut vec = kani_vec![1, 2];
        vec.push(3);
        assert!(vec == [1, 2, 3]);
    }

    push_test();
}
