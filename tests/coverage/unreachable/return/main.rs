// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn greet(is_guest: bool) -> &'static str {
    if is_guest {
        return "Welcome, Guest!";
    }
    // This part is unreachable if is_guest is true.
    "Hello, User!"
}

#[kani::proof]
fn main() {
    let is_guest = true;
    let message = greet(is_guest);
    assert_eq!(message, "Welcome, Guest!");
}
