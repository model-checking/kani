// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that the assert is reported as PARTIAL for having both REACHABLE and UNREACHABLE checks
fn any_bool() -> bool {
    kani::any()
}

#[kani::proof]
fn main() {
    if any_bool() {
        assert!(false);
    }

    if any_bool() {
        let s = "Fail with custom runtime message";
        assert!(false, "{}", s);
    }
}
