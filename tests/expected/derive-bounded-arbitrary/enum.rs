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
            for i in 0..=4 {
                kani::cover!(s.len() == i);
            }
        }
        Enum::B(v, _) => {
            for i in 0..=4 {
                kani::cover!(v.len() == i);
            }
        }
        Enum::C { x, y: _ } => {
            for i in 0..=4 {
                kani::cover!(x.len() == i);
            }
        }
    }
}
