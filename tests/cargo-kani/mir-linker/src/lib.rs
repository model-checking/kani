// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Dummy test to check --mir-linker runs on a cargo project.
use semver::{BuildMetadata, Prerelease, Version};

// Pre-CBMC 5.72.0, this test did not require an unwinding.
// TODO: figure out why it needs one now:
// https://github.com/model-checking/kani/issues/1978

#[kani::proof]
#[kani::unwind(2)]
fn check_version() {
    let next_major: u64 = kani::any();
    let next_minor: u64 = kani::any();
    kani::assume(next_major != 0 || next_minor != 0);

    // Check whether this requirement matches version 1.2.3-alpha.1 (no)
    let v0 = Version {
        major: 0,
        minor: 0,
        patch: 0,
        pre: Prerelease::EMPTY,
        build: BuildMetadata::EMPTY,
    };
    let next = Version {
        major: next_major,
        minor: next_minor,
        patch: 0,
        pre: Prerelease::EMPTY,
        build: BuildMetadata::EMPTY,
    };
    assert!(next > v0, "Next is greater");
}
