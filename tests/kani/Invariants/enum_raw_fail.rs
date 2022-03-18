// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-verify-fail
// Check that kani::raw_any for enums will generate invalid enums.
// This code should fail due to unreachable code being reached.

#[derive(Copy, Clone)]
enum Basic {
    Variant1,
    Variant2,
    Variant3,
}

#[kani::proof]
fn main() {
    let e = unsafe { kani::any_raw::<Basic>() };
    // This enum can be invalid and this code may actually not match any of the options below.
    // We had to split this into two matches because the compiler was statically pruning the
    // default branch of matches. When that happens, the failure is due to unreachable code being
    // executed.
    kani::expect_fail(
        matches!(e, Basic::Variant1 | Basic::Variant2) || matches!(e, Basic::Variant3),
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
