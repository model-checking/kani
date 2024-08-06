// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// Check that the message used in the kani::cover_or_fail macro appears in the results

fn foo(x: i32) -> Result<i32, &'static str> {
    if x < 100 { Ok(x / 2) } else { Err("x is too big") }
}

#[kani::proof]
#[kani::unwind(21)]
fn cover_match() {
    let x = kani::any();
    match foo(x) {
        Ok(y) if x > 20 => kani::cover_or_fail!(y > 20, "y may be greater than 20"), // satisfiable
        Ok(y) => kani::cover_or_fail!(y > 10, "y may be greater than 10"), // unsatisfiable
        Err(_s) => kani::cover_or_fail!(true, "foo may return Err"),       // satisfiable
    }
}
