// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that there's a compilation error if user tries to use Arbitrary when one of the fields do
//! not implement Arbitrary.

struct NotArbitrary(u8);

#[derive(kani::Arbitrary)]
struct Arbitrary(u8);

#[derive(kani::Arbitrary)]
struct Wrapper {
    arbitrary: Arbitrary,
    not_arbitrary: NotArbitrary,
}

#[kani::proof]
fn dead_harness() {
    panic!("This shouldn't compile");
}
