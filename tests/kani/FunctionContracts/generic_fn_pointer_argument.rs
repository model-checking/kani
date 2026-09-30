// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// `proof_for_contract` on a target whose type arguments include a fn-pointer type.
// `resolve_ty` now builds the fn-pointer type from a `Type::BareFn` argument (Rust ABI,
// non-variadic; lifetimes erased like every other path argument), so these resolve and
// verify:
//   * a plain fn-pointer argument (`fn(u8) -> u8`),
//   * a reference-carrying fn pointer (`fn(&u8) -> u8`), whose impl's stored type is
//     late-bound (`for<'a> fn(&'a u8) -> u8`); the erased argument matches it when no
//     `fn(&'static u8) -> u8` sibling impl exists (see generic_fn_pointer_higher_ranked.rs),
//   * an `unsafe` fn pointer (safety carried through the built signature).
// The two safe impls have distinct postconditions, so each harness only verifies if
// resolution picks *its* fn-pointer instantiation, not merely one of them.
// `generic_fn_pointer_cabi.rs` (expected suite) guards that a non-Rust-ABI fn pointer
// keeps the uninstantiated type and still fails to resolve.

struct Wrap<T>(T);

trait Probe {
    fn probe(&self) -> u32;
}

impl Probe for Wrap<fn(u8) -> u8> {
    #[kani::ensures(|r| *r == 0)]
    fn probe(&self) -> u32 {
        0
    }
}

// Distinct postcondition from the `fn(u8) -> u8` impl, so a harness only verifies if
// resolution lands on *this* fn-pointer instantiation rather than the sibling.
impl Probe for Wrap<fn(&u8) -> u8> {
    #[kani::ensures(|r| *r == 7)]
    fn probe(&self) -> u32 {
        7
    }
}

impl Probe for Wrap<unsafe fn(u8) -> u8> {
    #[kani::ensures(|r| *r == 3)]
    fn probe(&self) -> u32 {
        3
    }
}

mod verify {
    use super::*;

    #[kani::proof_for_contract(<Wrap<fn(u8) -> u8> as Probe>::probe)]
    fn check_plain_fn_pointer() {
        fn id(x: u8) -> u8 {
            x
        }
        let w = Wrap(id as fn(u8) -> u8);
        let _ = w.probe();
    }

    #[kani::proof_for_contract(<Wrap<fn(&u8) -> u8> as Probe>::probe)]
    fn check_reference_carrying_fn_pointer() {
        fn deref(x: &u8) -> u8 {
            *x
        }
        let w = Wrap(deref as fn(&u8) -> u8);
        let _ = w.probe();
    }

    #[kani::proof_for_contract(<Wrap<unsafe fn(u8) -> u8> as Probe>::probe)]
    fn check_unsafe_fn_pointer() {
        unsafe fn id(x: u8) -> u8 {
            x
        }
        let w = Wrap(id as unsafe fn(u8) -> u8);
        let _ = w.probe();
    }
}
