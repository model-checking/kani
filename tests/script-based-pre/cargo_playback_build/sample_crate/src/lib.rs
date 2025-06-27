// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test is used to test playback with a build configuration.

#[cfg(TARGET_OS = "linux")]
pub const OS_NAME: &'static str = "linux";

#[cfg(not(TARGET_OS = "linux"))]
pub const OS_NAME: &'static str = "other";

#[cfg(kani)]
mod harnesses {
    use super::*;

    #[kani::proof]
    fn harness() {
        kani::cover!(true, "Cover {OS_NAME}");
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn print_os_name() {
        println!("OS: {OS_NAME}");
        assert!(["linux", "other"].contains(&OS_NAME));
    }
}
