// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that derive BoundedArbitrary macro works on enums

#[allow(unused)]
#[derive(kani::BoundedArbitrary)]
enum Enum<T> {
    A(#[bounded] String),
    B(#[bounded] Vec<T>, usize),
    C {
        #[bounded]
        x: Vec<T>,
        y: bool,
    },
}

#[kani::proof]
#[kani::unwind(6)]
fn check_enum() {
    let any_enum: Enum<bool> = kani::bounded_any::<_, 4>();
    match any_enum {
        Enum::A(s) => {
            kani::cover!(s.len() == 0);
            kani::cover!(s.len() == 1);
            kani::cover!(s.len() == 2);
            kani::cover!(s.len() == 3);
            kani::cover!(s.len() == 4);
        }
        Enum::B(v, _) => {
            kani::cover!(v.len() == 0);
            kani::cover!(v.len() == 1);
            kani::cover!(v.len() == 2);
            kani::cover!(v.len() == 3);
            kani::cover!(v.len() == 4);
        }
        Enum::C { x, y: _ } => {
            kani::cover!(x.len() == 0);
            kani::cover!(x.len() == 1);
            kani::cover!(x.len() == 2);
            kani::cover!(x.len() == 3);
            kani::cover!(x.len() == 4);
        }
    }
}
