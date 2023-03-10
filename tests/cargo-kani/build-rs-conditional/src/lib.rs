// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This tests ensures that build scripts are able to conditionally check if they are running under
//! Kani.

#[cfg(kani)]
mod verify {
    /// Running `cargo kani` should verify that "RUNNING_KANI" is equals to "Yes"
    #[kani::proof]
    fn check() {
        assert_eq!(env!("RUNNING_KANI"), "Yes");
    }
}

#[cfg(test)]
mod test {
    /// Running `cargo test` should check that "RUNNING_KANI" is "No".
    #[test]
    fn check() {
        assert_eq!(env!("RUNNING_KANI"), "No");
    }
}
