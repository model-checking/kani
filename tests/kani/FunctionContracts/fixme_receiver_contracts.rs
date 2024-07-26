// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Checks that function contracts work with different types of receivers. I.e.:
//! - &Self (i.e. &self)
//! - &mut Self (i.e &mut self)
//! - Box<Self>
//! - Rc<Self>
//! - Arc<Self>
//! - Pin<P> where P is one of the types above
//! Source: <https://doc.rust-lang.org/reference/items/traits.html?highlight=receiver#object-safety>
// compile-flags: --edition 2021
// kani-flags: -Zfunction-contracts

#![feature(rustc_attrs)]

extern crate kani;

use std::boxed::Box;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

/// Type representing a valid ASCII value going from `0..=128`.
#[derive(Copy, Clone, PartialEq, Eq)]
#[rustc_layout_scalar_valid_range_start(0)]
#[rustc_layout_scalar_valid_range_end(128)]
struct CharASCII(u8);

impl kani::Arbitrary for CharASCII {
    fn any() -> CharASCII {
        let val = kani::any_where(|inner: &u8| *inner <= 128);
        unsafe { CharASCII(val) }
    }
}

/// This type contains unsafe setter functions with the same contract but different type of
/// receivers.
impl CharASCII {
    #[kani::modifies(&self.0)]
    #[kani::requires(new_val <= 128)]
    #[kani::ensures(|_| self.0 == new_val)]
    unsafe fn set_val(&mut self, new_val: u8) {
        self.0 = new_val
    }

    #[kani::modifies(&self.0)]
    #[kani::requires(new_val <= 128)]
    #[kani::ensures(|_| self.0 == new_val)]
    unsafe fn set_mut_ref(self: &mut Self, new_val: u8) {
        self.0 = new_val
    }

    #[kani::modifies(&self.as_ref().0)]
    #[kani::requires(new_val <= 128)]
    #[kani::ensures(|_| self.as_ref().0 == new_val)]
    unsafe fn set_box(mut self: Box<Self>, new_val: u8) {
        self.as_mut().0 = new_val
    }

    #[kani::modifies(&self.as_ref().0)]
    #[kani::requires(new_val <= 128)]
    #[kani::ensures(|_| self.as_ref().0 == new_val)]
    unsafe fn set_rc(mut self: Rc<Self>, new_val: u8) {
        Rc::<_>::get_mut(&mut self).unwrap().0 = new_val
    }

    #[kani::modifies(&self.as_ref().0)]
    #[kani::requires(new_val <= 128)]
    #[kani::ensures(|_| self.as_ref().0 == new_val)]
    unsafe fn set_arc(mut self: Arc<Self>, new_val: u8) {
        Arc::<_>::get_mut(&mut self).unwrap().0 = new_val;
    }

    #[kani::modifies(&self.0)]
    #[kani::requires(new_val <= 128)]
    #[kani::ensures(|_| self.0 == new_val)]
    unsafe fn set_pin(mut self: Pin<&mut Self>, new_val: u8) {
        self.0 = new_val
    }

    #[kani::modifies(&self.0)]
    #[kani::requires(new_val <= 128)]
    #[kani::ensures(|_| self.0 == new_val)]
    unsafe fn set_pin_box(mut self: Pin<Box<Self>>, new_val: u8) {
        self.0 = new_val
    }
}

mod verify {
    use super::*;
    use kani::Arbitrary;

    #[kani::proof_for_contract(CharASCII::set_val)]
    fn check_set_val() {
        let mut obj = CharASCII::any();
        let original = obj.0;
        let new_val = kani::any_where(|new| *new != original);
        unsafe { obj.set_val(new_val) };
    }

    #[kani::proof_for_contract(CharASCII::set_mut_ref)]
    fn check_mut_ref() {
        let mut obj = CharASCII::any();
        let original = obj.0;
        let new_val = kani::any_where(|new| *new != original);
        unsafe { obj.set_mut_ref(new_val) };
    }

    #[kani::proof_for_contract(CharASCII::set_box)]
    fn check_box() {
        let obj = CharASCII::any();
        let original = obj.0;
        let new_val = kani::any_where(|new| *new != original);
        unsafe { Box::new(obj).set_box(new_val) };
    }

    #[kani::proof_for_contract(CharASCII::set_rc)]
    fn check_rc() {
        let obj = CharASCII::any();
        let original = obj.0;
        let new_val = kani::any_where(|new| *new != original);
        unsafe { Rc::new(obj).set_rc(new_val) };
    }

    #[kani::proof_for_contract(CharASCII::set_arc)]
    fn check_arc() {
        let obj = CharASCII::any();
        let original = obj.0;
        let new_val = kani::any_where(|new| *new != original);
        unsafe { Arc::new(obj).set_arc(new_val) };
    }

    #[kani::proof_for_contract(CharASCII::set_pin)]
    fn check_pin() {
        let mut obj = CharASCII::any();
        let original = obj.0;
        let new_val = kani::any_where(|new| *new != original);
        unsafe { Pin::new(&mut obj).set_pin(new_val) };
    }

    #[kani::proof_for_contract(CharASCII::set_pin_box)]
    fn check_pin_box() {
        let obj = CharASCII::any();
        let original = obj.0;
        let new_val = kani::any_where(|new| *new != original);
        unsafe { Pin::new(Box::new(obj)).set_pin_box(new_val) };
    }
}
