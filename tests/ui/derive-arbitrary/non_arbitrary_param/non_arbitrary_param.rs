// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that Kani can automatically derive Arbitrary for structs with named fields.

struct Void;

#[derive(kani::Arbitrary)]
struct Wrapper<T> {
    inner: T,
}

#[kani::proof]
fn check_compile_error() {
    let _wrapper: Wrapper<Void> = kani::any();
    unreachable!("This shoulnd't compile");
}
