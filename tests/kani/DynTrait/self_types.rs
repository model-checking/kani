// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test all supported types for the self parameter in object-safe trait methods.
//!
//! See https://doc.rust-lang.org/reference/items/traits.html#object-safety for more details.

use std::boxed::Box;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

// Trait declaring methods with different possible types.
trait Trait {
    fn ref_id(&self) -> u8;
    fn mut_ref_id(self: &mut Self) -> u8;
    fn box_id(self: Box<Self>) -> u8;
    fn rc_id(self: Rc<Self>) -> u8;
    fn arc_id(self: Arc<Self>) -> u8;
    fn pin_id(self: Pin<&Self>) -> u8;
    fn pin_box_id(self: Pin<Box<Self>>) -> u8;
    fn replace_id(&mut self, new_id: u8) -> u8;
}

struct Concrete {
    id: u8,
}

// Trait implementation.
impl Trait for Concrete {
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

    fn pin_box_id(self: Pin<Box<Self>>) -> u8 {
        assert_eq!(self.as_ref().id, self.id);
        self.id
    }

    fn replace_id(&mut self, new_id: u8) -> u8 {
        std::mem::replace(&mut self.id, new_id)
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
    let ref_count: Rc<dyn Trait> = Rc::new(Concrete { id });
    assert_eq!(ref_count.ref_id(), id);
    assert_eq!(ref_count.rc_id(), id);
}

#[kani::proof]
pub fn check_arc() {
    let id = kani::any();
    let async_ref: Arc<dyn Trait> = Arc::new(Concrete { id });
    assert_eq!(async_ref.ref_id(), id);
    assert_eq!(async_ref.arc_id(), id);
}

#[kani::proof]
pub fn check_pin() {
    let id = kani::any();
    let obj = Concrete { id };
    let pin: Pin<&dyn Trait> = unsafe { Pin::new_unchecked(&obj) };
    assert_eq!(pin.ref_id(), id);
    assert_eq!(pin.pin_id(), id);
}

#[kani::proof]
pub fn check_pin_box() {
    let id = kani::any();
    let pin = Box::pin(Concrete { id });
    assert_eq!(pin.ref_id(), id);
    assert_eq!(pin.pin_box_id(), id);
}

#[kani::proof]
pub fn check_mut() {
    let initial_id = kani::any();
    let new_id = kani::any();
    let mut obj = Concrete { id: initial_id };
    let trt = &mut obj as &mut dyn Trait;
    assert_eq!(trt.replace_id(new_id), initial_id);
    assert_eq!(trt.ref_id(), new_id);
}
