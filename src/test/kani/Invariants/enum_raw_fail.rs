// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// rmc-verify-fail
// Check that rmc::raw_any for enums will generate invalid enums.
// This code should fail due to unreachable code being reached.

#[derive(Copy, Clone)]
enum Basic {
    Variant1,
    Variant2,
    Variant3,
}

fn main() {
    let e = unsafe { rmc::any_raw::<Basic>() };
    // This enum can be invalid and this code may actually not match any of the options below.
    rmc::expect_fail(
        matches!(e, Basic::Variant1 | Basic::Variant2 | Basic::Variant3),
        "Invalid enum variant",
    );
    match e {
        Basic::Variant1 => {
            let val = e as u8;
            assert!(val == 0);
            return;
        }
        Basic::Variant2 => {
            let val = e as u8;
            assert!(val == 1);
            return;
        }
        Basic::Variant3 => {
            let val = e as u8;
            assert!(val == 2);
            return;
        }
    }
}
