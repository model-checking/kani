// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we can verify a call to a boxed dyn Fn built from a function item.

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
    // `Hook::Custom(Box::new(hook))` builds the box the same way, so one call is enough.
    Hook::Default.into_box()(&2);
}
