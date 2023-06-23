// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that the verification summary printed at the end considers all
//! harnesses as "successfully verified".

#[kani::proof]
#[kani::should_panic]
fn harness1() {
    panic!("panicked on `harness1`!");
}

#[kani::proof]
#[kani::should_panic]
fn harness2() {
    panic!("panicked on `harness2`!");
}

#[kani::proof]
#[kani::should_panic]
fn harness3() {
    panic!("panicked on `harness3`!");
}
