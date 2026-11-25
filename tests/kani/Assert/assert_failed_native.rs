// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z unstable-options --no-assert-overrides

#[kani::proof]
#[kani::solver(cvc5)]
#[kani::should_panic]
fn check_failed_equality() {
    assert_eq!(Some(0u8), Some(1u8));
}

#[kani::proof]
#[kani::solver(cvc5)]
#[kani::should_panic]
fn check_failed_pattern() {
    std::assert_matches!(Some(0u8), Some(1));
}

mod panicking {
    #[inline(never)]
    pub fn assert_failed() -> u8 {
        1
    }
}

#[kani::proof]
fn check_success_skips_message() {
    let pointer = kani::any::<usize>() as *const u8;
    kani::assume(pointer.is_null());
    kani::cover!(true);
    assert_eq!(panicking::assert_failed(), 1, "unreachable {}", unsafe { *pointer });
}
