// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Ensure Kani can compile `[e; N]` when `N` is a const generic.
// Test from <https://github.com/model-checking/kani/issues/1728>

struct Foo<const N: usize> {
    field: [u64; N],
}

impl<const N: usize> Foo<N> {
    fn new() -> Self {
        // Was a crash during codegen because N hadn't been "monomorphized"
        // and so could not be evaluated to a compile-time constant.
        Self { field: [0; N] }
    }
}

#[cfg(kani)]
mod proofs {
    use super::*;

    #[kani::proof]
    fn hope_kani_does_not_crash() {
        let x = Foo::<32>::new();
        assert!(x.len() == 32);
    }
}
