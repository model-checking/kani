// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Testcase for https://github.com/model-checking/kani/issues/558.
// See https://rust-lang.github.io/unsafe-code-guidelines/layout/enums.html for information on the
// layout.
pub enum MyEnum {
    Val1,
    Val2,
}

fn foo(x: u32) -> Option<MyEnum> {
    // The math does overflow. Val1 == 0, Val2 == 1, None == 2.
    // The discriminant logic is val - max == 0 ? <> : <>; where max is 2
    if x > 10 { Some(MyEnum::Val2) } else { None }
}

#[kani::proof]
fn main() {
    let x = foo(15);
    assert!(x.is_some(), "assert");
}
