// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that derive BoundedArbitrary macro works on boxed slices, e.g. `Box<[u16]>`

extern crate kani;
use kani::BoundedArbitrary;

#[derive(BoundedArbitrary)]
#[allow(unused)]
struct StructWithBoxedSlice {
    x: i32,
    #[bounded]
    a: Box<[u16]>,
}

fn first(s: &[u16]) -> Option<u16> {
    if s.len() > 0 { Some(s[0]) } else { None }
}

fn tenth(s: &[u16]) -> Option<u16> {
    if s.len() >= 10 { Some(s[9]) } else { None }
}

#[kani::proof]
#[kani::unwind(11)]
fn check_my_boxed_array() {
    let swbs: StructWithBoxedSlice = kani::bounded_any::<_, 10>();
    let f = first(&swbs.a);
    kani::cover!(f.is_none());
    kani::cover!(f == Some(1));
    kani::cover!(f == Some(42));
    let t = tenth(&swbs.a);
    kani::cover!(t.is_none());
    kani::cover!(t == Some(15));
    kani::cover!(t == Some(987));
}
