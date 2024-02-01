// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

use std::ops::Deref;
/// Illustrates the problem from https://github.com/model-checking/kani/issues/2907
use std::rc::Rc;

#[kani::modifies({
    let intref : &u32  = ptr.deref().deref();
    intref
})]
fn modify(ptr: Rc<&mut u32>) {
    unsafe {
        **(Rc::as_ptr(&ptr) as *mut &mut u32) = 1;
    }
}

#[kani::proof_for_contract(modify)]
fn main() {
    let mut i: u32 = kani::any();
    let ptr = Rc::new(&mut i);
    modify(ptr.clone());
}

#[kani::proof]
#[kani::stub_verified(modify)]
fn replace_modify() {
    let begin = kani::any_where(|i| *i < 100);
    let i = Rc::new(RefCell::new(begin));
    modify(i.clone());
    kani::assert(*i.borrow() == begin + 1, "end");
}
