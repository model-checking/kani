// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that the `unexpected_cfgs` lint (enabled by default as of
// the 2024-05-05 toolchain) does not cause `cargo kani` to emit warnings when
// the code has `#[cfg(kani)]`. Kani avoids the warning by adding
// `--check-cfg=cfg(kani)` to the rust flags.

#![deny(unexpected_cfgs)]

fn main() {}

#[cfg(kani)]
mod kani_checks {
    #[kani::proof]
    fn check_unexpected_cfg() {
        assert_eq!(1, 1);
    }
}
