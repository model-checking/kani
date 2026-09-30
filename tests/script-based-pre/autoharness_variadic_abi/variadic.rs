// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A C-variadic whose calling convention is not the C one cannot be modeled at all, so autoharness
//! names the calling convention rather than blaming the `VaList` argument for lacking an
//! `Arbitrary` implementation: https://github.com/model-checking/kani/issues/4817

#[unsafe(naked)]
pub unsafe extern "sysv64" fn variadic_sysv64(_: ...) -> u32 {
    core::arch::naked_asm!("")
}
