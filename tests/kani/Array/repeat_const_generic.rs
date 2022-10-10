// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Ensure Kani can evaluate `[e; N]` when `N` is a const generic

struct Foo<const N: usize> {
    field: [u64; N],
}

impl<const N: usize> Foo<N> {
    fn new() -> Self {
        Self { field: [0; N] }
    }
}

#[cfg(kani)]
mod proofs {
    use super::*;

    #[kani::proof]
    fn kani_crash() {
        Foo::<32>::new();
    }
}
