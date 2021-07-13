// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// cbmc-flags: --unwind 11

include!("../../rmc-prelude.rs");

fn main() {
    let mut a: u32 = __nondet();

    if a < 1024 {
        while a > 0 {
            a = a / 2;
        }

        assert!(a == 0);
    }
}
