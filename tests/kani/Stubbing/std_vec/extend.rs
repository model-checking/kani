// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --use-abs --abs-type kani
fn main() {
    fn extend_test() {
        let mut vec = Vec::new();
        vec.push(1);
        vec.push(2);

        assert!(vec.len() == 2);
        assert!(vec[0] == 1);

        assert!(vec.pop() == Some(2));
        assert!(vec.len() == 1);

        vec[0] = 7;
        assert!(vec[0] == 7);

        vec.extend([1, 2, 3]);

        assert!(vec == [7, 1, 2, 3]);
    }

    extend_test();
}
