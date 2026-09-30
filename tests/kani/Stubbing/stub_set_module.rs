// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! Test stub sets defined in a submodule.
//! Paths in stub sets are resolved relative to the harness, so use
//! paths that are valid from the harness's scope.

fn real_fn() -> u32 {
    0
}

fn stub_fn() -> u32 {
    42
}

mod stubs {
    kani::stub_set!(pub my_set,
        stub(real_fn, stub_fn),
    );
}

#[kani::proof]
#[kani::use_stub_set(stubs::my_set)]
fn check_stub_set_from_module() {
    assert_eq!(real_fn(), 42);
}
