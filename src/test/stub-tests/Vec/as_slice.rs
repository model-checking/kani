// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-flags: --use-abs --abs-type rmc
fn main() {
    fn as_slice_test() {
        use std::io::{self, Write};
        let buffer = rmc_vec![1, 2, 3, 5, 8];
        io::sink().write(buffer.as_slice()).unwrap();
    }

    as_slice_test();
}
