// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --use-abs --abs-type kani
fn main() {
    fn is_empty_test() {
        let mut v = Vec::new();
        assert!(v.is_empty());

        v.push(1);
        assert!(!v.is_empty());
    }

    is_empty_test();
}
