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
    fn value_id(self: Self) -> u8;
    fn ref_id(&self) -> u8;
    fn mut_ref_id(self: &mut Self) -> u8;
    fn box_id(self: Box<Self>) -> u8;
    fn rc_id(self: Rc<Self>) -> u8;
    fn arc_id(self: Arc<Self>) -> u8;
    fn pin_id(self: Pin<&Self>) -> u8;
    fn nested_id(self: Pin<Arc<Rc<Box<Self>>>>) -> u8;
}

struct Concrete {
    id: u8,
}

impl Trait for Concrete {
    fn value_id(self: Self) -> u8 {
        self.id
    }

    fn ref_id(&self) -> u8 {
        self.id
    }

    fn mut_ref_id(self: &mut Self) -> u8 {
        self.id
    }

    fn box_id(self: Box<Self>) -> u8 {
        assert_eq!(self.as_ref().id, self.id);
        self.id
    }

    fn rc_id(self: Rc<Self>) -> u8 {
        assert_eq!(self.as_ref().id, self.id);
        self.id
    }

    fn arc_id(self: Arc<Self>) -> u8 {
        assert_eq!(self.as_ref().id, self.id);
        self.id
    }

    fn pin_id(self: Pin<&Self>) -> u8 {
        assert_eq!(self.as_ref().id, self.id);
        self.id
    }

    fn nested_id(self: Pin<Arc<Rc<Box<Self>>>>) -> u8 {
        assert_eq!(self.as_ref().as_ref().as_ref().as_ref().id, self.id);
        self.id
    }
}

#[kani::proof]
pub fn check_box() {
    let id = kani::any();
    let boxed: Box<dyn Trait> = Box::new(Concrete { id });
    assert_eq!(boxed.ref_id(), id);
    assert_eq!(boxed.box_id(), id);
}

#[kani::proof]
pub fn check_rc() {
    let id = kani::any();
    let boxed: Rc<dyn Trait> = Rc::new(Concrete { id });
    assert_eq!(boxed.rc_id(), id);
}

#[kani::proof]
pub fn check_pin() {
    let id = kani::any();
    let obj = Concrete { id };
    let pin: Pin<&dyn Trait> = unsafe { Pin::new_unchecked(&obj) };
    assert_eq!(pin.pin_id(), id);
    assert_eq!(pin.ref_id(), id);
}
