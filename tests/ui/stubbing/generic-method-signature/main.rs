// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z stubbing

trait Pick {
    fn pick<T: Copy>(&self, value: T) -> bool;
}
struct S;
impl Pick for S {
    fn pick<T: Copy>(&self, _: T) -> bool {
        false
    }
}
fn incompatible(_: &S, _: u8) -> bool {
    true
}

#[kani::proof]
#[kani::stub(<S as Pick>::pick, incompatible)]
fn bad_signature() {
    assert!(S.pick(1u8));
}
