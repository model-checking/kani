// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --only-codegen
//
// Check that we can codegen a boxed dyn Fn built from a function item.
// A function item coerces through a zero-sized singleton, so the pointer stored in the
// box is a bare pointer to that singleton rather than one already shaped like the field.

fn hook(x: &i32) {
    assert!(*x == 2);
}

enum Hook {
    Default,
    Custom(Box<dyn Fn(&i32) + Send + Sync + 'static>),
}

impl Hook {
    fn into_box(self) -> Box<dyn Fn(&i32) + Send + Sync + 'static> {
        match self {
            Hook::Default => Box::new(hook),
            Hook::Custom(hook) => hook,
        }
    }
}

#[kani::proof]
fn main() {
    Hook::Default.into_box()(&2);
    Hook::Custom(Box::new(hook)).into_box()(&2);
}
