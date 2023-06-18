// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

#[kani::ensures(kani::old(ptr) == *ptr - 1)]
fn modify(ptr: &mut u32) -> u32 {
    *ptr += 1;
    0
}

#[kani::proof]
fn main() {
    modify(&mut 0);
}