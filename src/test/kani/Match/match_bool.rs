// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// compile-flags: --crate-type lib
// rmc-flags: --function match_bool

#[rmc::proof]
pub fn match_bool() {
    let arg: bool = rmc::nondet();
    let var = match arg {
        true => !arg,
        _ => arg,
    };

    let var2 = match arg {
        _ => false,
    };

    assert!(var == var2);

    let mut i = 0;

    match arg {
        true => i = 1,
        false => i = 2,
        _ => i = 3,
    }

    assert!(i == 1 || i == 2);
}
