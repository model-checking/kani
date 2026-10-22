// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// `proof_for_contract` on a target whose type arguments include a trait object.
// `resolve_ty` builds `dyn Trait` from a `Type::TraitObject` argument (one
// principal trait plus auto traits; lifetimes erased), so these resolve and verify:
//   * a boxed principal (`Box<dyn TA>`),
//   * a sibling on a different principal (`Box<dyn TB>`) with a distinct
//     postcondition, so each harness only verifies against its own impl,
//   * an auto-trait variant (`Box<dyn TA + Send>`) — a distinct type from
//     `Box<dyn TA>`, discriminating the predicate list,
//   * a generic principal (`Box<dyn Get<u8>>`) — the trait's own argument resolves.
// generic_trait_object_binding.rs (expected suite) guards that an
// associated-type binding keeps the uninstantiated type and fails to resolve.

use std::boxed::Box;
use std::marker::Send;

trait TA {
    fn a(&self) -> u8;
}
trait TB {
    fn b(&self) -> u8;
}
trait Get<T> {
    fn g(&self) -> T;
}

struct S<T>(T);
trait Probe {
    fn probe(&self) -> u32;
}

impl Probe for S<Box<dyn TA>> {
    #[kani::ensures(|r| *r == 1)]
    fn probe(&self) -> u32 {
        1
    }
}

// Distinct postcondition: resolution must pick THIS impl, not the TA sibling.
impl Probe for S<Box<dyn TB>> {
    #[kani::ensures(|r| *r == 2)]
    fn probe(&self) -> u32 {
        2
    }
}

// `dyn TA + Send` is a different type than `dyn TA`: predicate-list discrimination.
impl Probe for S<Box<dyn TA + Send>> {
    #[kani::ensures(|r| *r == 3)]
    fn probe(&self) -> u32 {
        3
    }
}

// Generic principal: the trait's own type argument resolves recursively.
impl Probe for S<Box<dyn Get<u8>>> {
    #[kani::ensures(|r| *r == 4)]
    fn probe(&self) -> u32 {
        4
    }
}

struct ImplA;
impl TA for ImplA {
    fn a(&self) -> u8 {
        0
    }
}
struct ImplB;
impl TB for ImplB {
    fn b(&self) -> u8 {
        0
    }
}
struct ImplG;
impl Get<u8> for ImplG {
    fn g(&self) -> u8 {
        0
    }
}

mod verify {
    use super::*;

    #[kani::proof_for_contract(<S<Box<dyn TA>> as Probe>::probe)]
    fn check_dyn_ta() {
        let s = S(Box::new(ImplA) as Box<dyn TA>);
        let _ = s.probe();
    }

    #[kani::proof_for_contract(<S<Box<dyn TB>> as Probe>::probe)]
    fn check_dyn_tb() {
        let s = S(Box::new(ImplB) as Box<dyn TB>);
        let _ = s.probe();
    }

    #[kani::proof_for_contract(<S<Box<dyn TA + Send>> as Probe>::probe)]
    fn check_dyn_ta_send() {
        let s = S(Box::new(ImplA) as Box<dyn TA + Send>);
        let _ = s.probe();
    }

    #[kani::proof_for_contract(<S<Box<dyn Get<u8>>> as Probe>::probe)]
    fn check_dyn_get_u8() {
        let s = S(Box::new(ImplG) as Box<dyn Get<u8>>);
        let _ = s.probe();
    }
}
