// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// cbmc-flags: --unwind 10

fn main() {
    let mut a: u32 = rmc::nondet();

    if a < 1024 {
        loop {
            a = a / 2;

            if a == 0 {
                break;
            }
        }

        assert!(a == 0);
    }
}
