// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

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
        PrivType::<i32>::new().priv_fn()
    }
}

enum PrivType<T> {
    Empty(std::marker::PhantomData<T>),
}

impl<T> PrivType<T> {
    fn new() -> Self {
        Self::Empty(std::marker::PhantomData)
    }

    fn priv_fn(&self) -> u32 {
        0
    }

    fn the_answer(&self) -> u32 {
        42
    }
}
