// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Crate that defines a method to be stubbed as well its stubs.

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum StubConfig {
    NoStub,
    Stub1,
    Stub2,
}

impl StubConfig {
    pub fn new() -> StubConfig {
        StubConfig::NoStub
    }
}

pub mod stubs {
    use super::*;

    pub fn new_stub_1() -> StubConfig {
        StubConfig::Stub1
    }

    pub fn new_stub_2() -> StubConfig {
        StubConfig::Stub2
    }
}

#[cfg(kani)]
mod verify {
    use super::*;

    #[kani::proof]
    #[kani::stub(StubConfig::new, stubs::new_stub_1)]
    fn check_stub_1() {
        let config = StubConfig::new();
        assert_eq!(config, StubConfig::Stub1);
    }

    #[kani::proof]
    fn check_no_stub() {
        let config = StubConfig::new();
        assert_eq!(config, StubConfig::NoStub);
    }
}
