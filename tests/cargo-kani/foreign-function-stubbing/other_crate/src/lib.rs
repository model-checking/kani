// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub fn pub_fn() -> u32 {
    0
}

pub fn fn_delegating_to_priv_fn() -> u32 {
    priv_fn()
}

fn priv_fn() -> u32 {
    0
}

fn the_answer() -> u32 {
    42
}

pub struct PubType {}

impl PubType {
    pub fn new() -> Self {
        Self {}
    }

    pub fn pub_fn(&self) -> u32 {
        0
    }

    pub fn fn_delegating_to_priv_fn(&self) -> u32 {
        self.priv_fn()
    }

    fn priv_fn(&self) -> u32 {
        0
    }

    fn the_answer(&self) -> u32 {
        42
    }

    pub fn fn_delegating_to_priv_type(&self) -> u32 {
        PrivType::new().priv_fn()
    }
}

struct PrivType {}

impl PrivType {
    fn new() -> Self {
        Self {}
    }

    fn priv_fn(&self) -> u32 {
        0
    }

    fn the_answer(&self) -> u32 {
        42
    }
}

pub mod pub_mod {
    pub fn pub_fn() -> u32 {
        0
    }

    pub fn fn_delegating_to_priv_fn() -> u32 {
        priv_mod::pub_fn()
    }

    mod priv_mod {
        pub fn pub_fn() -> u32 {
            0
        }

        fn the_answer() -> u32 {
            42
        }
    }
}
