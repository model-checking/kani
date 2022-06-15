// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test all supported types for the self parameter in methods (with no recursion). Grammar for
//! types is defined here: https://doc.rust-lang.org/reference/items/associated-items.html#methods
//! P = &'lt S | &'lt mut S | Box<S> | Rc<S> | Arc<S> | Pin<P>
//! S = Self | P

use std::boxed::Box;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

trait Trait {
    fn pin_id(self: Pin<&Self>) -> u8;
    fn id(&self) -> u8;
}

struct Concrete {
    id: u8,
}

impl Trait for Concrete {
    fn pin_id(self: Pin<&Self>) -> u8 {
        assert_eq!(self.get_ref().id, self.id);
        self.id
    }

    fn id(&self) -> u8 {
        self.id
    }
}

#[kani::proof]
pub fn check_pin() {
    let id = kani::any();
    let obj = Concrete { id };
    let pin: Pin<&dyn Trait> = unsafe { Pin::new_unchecked(&obj) };
    assert_eq!(pin.pin_id(), id);
    assert_eq!(pin.id(), id);
}
