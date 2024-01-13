// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --use-abs --abs-type kani
fn main() {
    fn into_iter_test() {
        let v = kani_vec![1, 4, 5];
        let mut iter = v.into_iter();

        assert!(iter.next() == Some(1));
        assert!(iter.next() == Some(4));
        assert!(iter.next() == Some(5));
        assert!(iter.next() == None);
    }

    into_iter_test();
}
