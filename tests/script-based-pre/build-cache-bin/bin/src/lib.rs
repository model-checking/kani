// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#[kani::proof]
fn cover_bool() {
    match kani::any() {
        true => kani::cover!(true, "true"),
        false => kani::cover!(true, "false"),
    }
}

#[kani::proof]
fn cover_option() {
    match kani::any() {
        Some(true) => kani::cover!(true, "true"),
        Some(false) => kani::cover!(true, "false"),
        None => kani::cover!(true, "none"),
    }
}
