// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! C-variadics are supported (c.f. tests/kani/FunctionCall/Variadic); autoharness still cannot
//! generate their `VaList` argument, which is the honest reason to report for them.

pub unsafe extern "C" fn variadic_c(a: u32, _: ...) -> u32 {
    a
}

pub fn double(x: u8) -> u16 {
    u16::from(x) * 2
}
