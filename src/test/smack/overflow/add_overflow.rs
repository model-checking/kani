// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// @flag --integer-overflow
// @expect overflow
// rmc-verify-fail

pub fn get128() -> u8 {
    128
}

pub fn main() {
    let a: u8 = get128();
    let b: u8 = get128();
    let c = a + b;
}
