// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that our macro override supports different types of messages.
#[kani::proof]
fn check_unreachable() {
    let msg = "Oops.";
    match kani::any::<u8>() {
        0 => unreachable!(),
        1 => unreachable!("Error message"),
        2 => unreachable!("Unreachable message with arg {}", "str"),
        3 => unreachable!("{}", msg),
        _ => unreachable!(msg),
    }
}
