// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --use-abs --abs-type kani
fn main() {
    fn shrink_to_test() {
        let mut vec = Vec::with_capacity(10);
        vec.extend([1, 2, 3]);
        assert!(vec.capacity() == 10);
        vec.shrink_to(4);
        assert!(vec.capacity() >= 4);
        vec.shrink_to(0);
        kani::expect_fail(vec.capacity() >= 3, "Capacity shrinked to 0");
    }

    shrink_to_test()
}
