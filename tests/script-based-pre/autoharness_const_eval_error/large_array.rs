// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Variant of tests/ui/ice-size-overflow for autoharness. With `--reachability=all_fns`, the
//! reachability collector visits the body of `<S<u8> as SizedTypeProperties>::SIZE`'s user
//! and finds a constant whose evaluation failed with E0080. This used to trigger an ICE
//! ("Instance with polymorphic constant"). See
//! <https://github.com/model-checking/kani/issues/4814>.

struct S<T> {
    x: [T; !0],
}

pub fn f() -> usize {
    std::mem::size_of::<S<u8>>()
}

fn main() {
    let _x = f();
}
