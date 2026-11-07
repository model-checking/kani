// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Check that `kani list` attaches a contract harness to its target when the module that
//! holds the harness is declared before the function it verifies (#5099).

pub mod dur;
