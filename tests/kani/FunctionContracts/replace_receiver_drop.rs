// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts -Zstubbing

//! When an owned-receiver contract is used in replace mode, the receiver is
//! moved into the replace closure and dropped there (see
//! const_fn_owned_receiver.rs for why the move exists). Pin what that move
//! promises: the receiver's destructor still runs, exactly once. Skipping it
//! (a `mem::forget` regression) or doubling it fails the counter check. The
//! drop's position among the stub's other locals is deliberately NOT pinned --
//! the real body controls its own drops, so the stub cannot promise an order.
//! The function is non-`const`, so this also covers the ordinary-method replace
//! path (the move lands on every owned-receiver replace, not just `const` ones).

use std::sync::atomic::{AtomicUsize, Ordering};

static DROPS: AtomicUsize = AtomicUsize::new(0);

struct Owned(u32);

impl Drop for Owned {
    fn drop(&mut self) {
        DROPS.fetch_add(1, Ordering::Relaxed);
    }
}

impl Owned {
    #[kani::requires(self.0 < 100)]
    #[kani::ensures(|result: &u32| *result == old(self.0))]
    fn into_value(self) -> u32 {
        self.0
    }
}

/// Required companion to the `stub_verified` harness below: Kani only accepts a
/// verified stub when a `proof_for_contract` proves the contract holds. It does
/// not assert the drop count -- Kani's contract-check instrumentation models the
/// real body's own drops differently from the replace stub, and that path is not
/// what this test guards.
#[kani::proof_for_contract(Owned::into_value)]
fn check_owned_receiver_contract() {
    let v: u32 = kani::any_where(|v: &u32| *v < 100);
    let out = Owned(v).into_value();
    assert!(out == v);
}

/// The stub, not the real body, drops the receiver -- exactly once, by the time
/// the call returns. The cover proves the path is live, so the asserts cannot
/// pass vacuously through a pruned assume.
#[kani::proof]
#[kani::stub_verified(Owned::into_value)]
fn check_replace_drops_receiver_exactly_once() {
    let v: u32 = kani::any_where(|v: &u32| *v < 100);
    let out = Owned(v).into_value();
    assert!(out == v);
    assert!(DROPS.load(Ordering::Relaxed) == 1);
    kani::cover!(true, "post-call path is live");
}
