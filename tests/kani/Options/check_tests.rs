// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we can verify test harnesses using the --tests argument.
// Note: We need to provide the compile-flags because compile test runs rustc directly and via kani.

// compile-flags: --test
// kani-flags: --tests

pub mod my_mod {
    pub fn fn_under_verification(a: i32) {
        assert!(a > 0);
    }
}

#[cfg(test)]
mod test {
    use my_mod::fn_under_verification;

    #[test]
    #[kani::proof]
    fn test_harness() {
        let input: i32 = kani::nondet();
        kani::assume(input > 1);
        fn_under_verification(input);
    }
}
