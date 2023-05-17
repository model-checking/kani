// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that running the unit test generated using the concrete playback feature
// with `RUSTFLAGS="--cfg=kani" cargo +nightly test doesn't cause a compilation error.
// There is an existing UI test to generate the unit test itself (in kani/tests/ui/concrete-playback/result).

#[cfg(kani)]
mod verify {
    #[kani::proof]
    fn any_is_ok() {
        let result: Result<char, bool> = kani::any();
        kani::cover!(result.is_ok());
    }

    #[kani::proof]
    fn any_is_err() {
        let result: Result<char, bool> = kani::any();
        kani::cover!(result.is_err());
    }
}
