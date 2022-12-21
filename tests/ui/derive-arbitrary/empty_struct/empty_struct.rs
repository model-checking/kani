// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that Kani can automatically derive Arbitrary for empty structs.

#[derive(kani::Arbitrary)]
struct Void;

#[derive(kani::Arbitrary)]
struct Void2(());

#[derive(kani::Arbitrary)]
struct VoidOfVoid(Void, Void2);

#[kani::proof]
fn check_arbitrary_point() {
    let _v: VoidOfVoid = kani::any();
}
