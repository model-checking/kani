// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check the location of the assert statement when using assert!(false);

fn any_bool() -> bool {
    kani::nondet()
}

pub fn main() {
    if any_bool() {
        assert!(false);
    }

    if any_bool() {
        let s = "Fail with custom runtime message";
        assert!(false, "{}", s);
    }

    if any_bool() {
        assert!(false, "Fail with custom static message");
    }
}

#[inline(always)]
#[track_caller]
fn check_caller(b: bool) {
    assert!(b);
}
