// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that `--concrete-playback=inplace` works when Kani runs inside a workspace member.
//! The compiler records this file as `member/src/lib.rs`, relative to the workspace root.

pub fn first(v: &[u8]) -> Option<u8> {
    v.first().copied()
}

#[cfg(kani)]
mod harnesses {
    use super::*;

    #[kani::proof]
    fn harness() {
        let v: [u8; 1] = kani::any();
        kani::cover!(first(&v) == Some(42), "first is 42");
    }
}
