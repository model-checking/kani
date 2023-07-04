// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This tests ensures that build scripts are able to conditionally check if they are running under
//! Kani (in both verification and playback mode).

#[cfg(kani)]
mod verify {
    /// Running `cargo kani` should verify that "RUNNING_KANI" is equals to "Yes"
    #[kani::proof]
    fn check_kani() {
        assert_eq!(env!("RUNNING_KANI"), "Yes");
        // Add a dummy cover so playback generates a test that should succeed.
        kani::cover!(kani::any());
    }
}

#[cfg(test)]
mod test {
    /// Running `cargo test` should check that "RUNNING_KANI" is "No".
    #[test]
    fn check_not_kani() {
        assert_eq!(env!("RUNNING_KANI"), "No");
    }
}
