// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![no_std]

pub fn foo(_x: &[usize]) -> core::ops::Range<usize> {
    core::ops::Range { start: 5, end: 25 }
}

pub fn bar(r: core::ops::Range<usize>) -> core::ops::Range<usize> {
    let a = [1, 2, 3];
    let b = &a[..1];
    core::ops::Range { start: r.start + b[0], end: r.end }
}
