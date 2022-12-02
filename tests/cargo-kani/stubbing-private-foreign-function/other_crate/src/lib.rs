// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub fn fn_delegating_to_priv_fn() -> u32 {
    priv_fn()
}

fn priv_fn() -> u32 {
    0
}

fn the_answer() -> u32 {
    42
}

pub mod pub_mod {
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
