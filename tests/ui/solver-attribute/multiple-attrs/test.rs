// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
#[kani::solver("solver1")]
#[kani::solver("solver2")]
fn check() {}
