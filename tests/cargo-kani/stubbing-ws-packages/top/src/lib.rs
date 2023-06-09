// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Crate that defines a method to be stubbed as well its stubs.

use dependency::StubConfig;

pub fn stub_id() -> Option<u8> {
    let config = StubConfig::new();
    match config {
        StubConfig::NoStub => None,
        StubConfig::Stub1 => Some(1),
        StubConfig::Stub2 => Some(2),
    }
}

#[cfg(kani)]
mod verify_top {
    use super::stub_id;
    use dependency::*;

    #[kani::proof]
    #[kani::stub(StubConfig::new, stubs::new_stub_1)]
    fn check_stub_1() {
        assert_eq!(stub_id(), Some(1));
    }

    #[kani::proof]
    #[kani::stub(StubConfig::new, stubs::new_stub_2)]
    fn check_stub_2() {
        assert_eq!(stub_id(), Some(2));
    }

    #[kani::proof]
    fn check_no_stub() {
        assert_eq!(stub_id(), None);
    }
}
