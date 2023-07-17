// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check the location of the assert statement when using assert!(false);
// This currently requires the old format since the new format doesn't include
// line numbers https://github.com/model-checking/kani/issues/918
// Also disable reachability checks because they add annotations to each
// assert's description which would be visible with the old output format

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
        kani::cover!();
        assert!(false, "{}", s);
    }

    if any_bool() {
        kani::cover!();
        assert!(false, "Fail with custom static message");
    }
}

#[inline(always)]
#[track_caller]
fn check_caller(b: bool) {
    assert!(b);
}
