// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --use-abs --abs-type kani
fn main() {
    fn as_mut_slice_test() {
        let mut buffer = kani_vec![1, 2, 3];
        buffer.as_mut_slice().reverse();
        assert!(buffer == [3, 2, 1]);
    }

    as_mut_slice_test();
}
