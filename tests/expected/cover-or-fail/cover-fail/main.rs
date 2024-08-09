// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// Check that overconstraining could lead to unsatisfiable cover statements

enum Sign {
    Positive,
    Negative,
    Zero,
}

fn get_sign(x: i32) -> Sign {
    if x > 0 {
        Sign::Positive
    } else if x < 0 {
        Sign::Negative
    } else {
        Sign::Zero
    }
}

#[kani::proof]
fn cover_overconstrained() {
    let x: i32 = kani::any();
    let sign = get_sign(x);

    match sign {
        Sign::Zero => kani::cover_or_fail!(x != 0),
        _ => {}
    }
}
