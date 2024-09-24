// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// compile-flags: --edition 2018

// Regression test for https://github.com/model-checking/kani/issues/1593
// The problem was that the size of a coroutine was wrong, which was discovered
// in the context of vtables.

use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};

#[kani::proof]
fn issue_1593() {
    let x = Arc::new(AtomicI64::new(0));
    let x2 = x.clone();
    let gen = async move {
        async {}.await;
        x2.fetch_add(1, Ordering::Relaxed);
    };
    assert_eq!(std::mem::size_of_val(&gen), 16);
    let pinbox = Box::pin(gen); // check that vtables work
}
