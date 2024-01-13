// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --use-abs --abs-type kani
fn main() {
    fn as_slice_test() {
        use std::io::{self, Write};
        let buffer = kani_vec![1, 2, 3, 5, 8];
        io::sink().write(buffer.as_slice()).unwrap();
    }

    as_slice_test();
}
