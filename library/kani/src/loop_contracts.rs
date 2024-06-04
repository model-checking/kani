// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// This function is only used for loop contract annotation.
/// It behaves as a placeholder to telling us where the loop invariants stmts begin.
#[inline(never)]
#[rustc_diagnostic_item = "KaniLoopInvariantBegin"]
#[doc(hidden)]
#[crate::unstable(
    feature = "loop-contracts",
    issue = 3168,
    reason = "experimental loop contracts support"
)]
pub const fn kani_loop_invariant_begin_marker() {}

/// This function is only used for loop contract annotation.
/// It behaves as a placeholder to telling us where the loop invariants stmts end.
#[inline(never)]
#[rustc_diagnostic_item = "KaniLoopInvariantEnd"]
#[doc(hidden)]
#[crate::unstable(
    feature = "loop-contracts",
    issue = 3168,
    reason = "experimental loop contracts support"
)]
pub const fn kani_loop_invariant_end_marker() {}
