// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --use-abs --abs-type kani
fn main() {
    fn shrink_to_fit_test() {
        let mut vec = Vec::with_capacity(10);
        vec.extend([1, 2, 3]);
        assert_eq!(vec.capacity(), 10);
        vec.shrink_to_fit();
        assert!(vec.capacity() >= 3);
    }

    shrink_to_fit_test();
}
