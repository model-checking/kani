// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --use-abs --abs-type kani
fn main() {
    fn clear_test() {
        let mut v = kani_vec![1, 2, 3];

        v.clear();

        assert!(v.is_empty());
    }

    clear_test();
}
