// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --use-abs --abs-type kani
fn main() {
    fn reserve_test() {
        let mut vec = kani_vec![1];
        vec.reserve(10);
        assert!(vec.capacity() >= 11);
    }

    reserve_test();
}
