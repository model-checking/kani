// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// A crate with global ASM

// defines a function `foo`
std::arch::global_asm!(".global foo", "foo:", "nop",);

pub static mut STATIC_VAR: u16 = 98;

// exports the fn `foo`
extern "C" {
    pub fn foo();
}

// a function that does not involve any ASM
pub fn eleven() -> i32 {
    11
}
