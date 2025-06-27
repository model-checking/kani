// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that invalid attributes are caught for all crate items

#[kani::stub(invalid=opt)]
pub fn unreachable_fn() {}

// Also gracefully handle user embedded kanitool.
#[kanitool::proof(invalid_argument)]
#[kanitool::invalid::attribute]
pub fn invalid_kanitool() {}

#[kani::proof]
pub fn valid_harness() {}
