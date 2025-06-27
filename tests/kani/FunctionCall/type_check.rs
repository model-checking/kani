// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Test that Kani can properly handle the ABI of virtual calls with ZST arguments.
//! Issue first reported here: <https://github.com/model-checking/kani/issues/2312>
use std::any::TypeId;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

struct MyError;

impl Debug for MyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Display for MyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Error for MyError {}

#[kani::proof]
fn is_same_error() {
    let e = MyError;
    let d = &e as &(dyn Error);
    assert!(d.is::<MyError>());
    assert!(!d.is::<std::io::Error>());
}
