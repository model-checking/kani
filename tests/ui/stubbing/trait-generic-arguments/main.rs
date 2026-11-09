// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z stubbing

trait Generic<T> {
    fn method<U: Copy>(&self, value: T, extra: U) -> bool;
}
struct Target;
impl Generic<u8> for Target {
    fn method<U: Copy>(&self, _: u8, _: U) -> bool {
        false
    }
}
fn replacement<U: Copy>(_: &Target, _: u8, _: U) -> bool {
    true
}

#[kani::proof]
#[kani::stub(<Target as Generic>::method, replacement)]
fn missing_type_argument() {
    let _ = Target.method(0u8, 1u64);
}

#[kani::proof]
#[kani::stub(<Target as Generic<u8, u16>>::method, replacement)]
fn extra_type_argument() {
    let _ = Target.method(0u8, 1u64);
}

#[kani::proof]
#[kani::stub(<Target as Generic<u16>>::method, replacement)]
fn missing_implementation() {
    let _ = Target.method(0u8, 1u64);
}
