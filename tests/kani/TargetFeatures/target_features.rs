// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// This test checks that the Kani compiler propertly enables the
/// architecture-specific target features (e.g. `neon` on `aarch64` and
/// `sse`/`sse2` on `x86_64`)

#[kani::proof]
fn check_expected_target_features() {
    #[cfg(target_arch = "aarch64")]
    {
        assert!(cfg!(target_feature = "neon"));
    }

    #[cfg(target_arch = "x86_64")]
    {
        assert!(cfg!(target_feature = "sse"));
        assert!(cfg!(target_feature = "sse2"));
        assert!(cfg!(target_feature = "x87"));
    }
}
