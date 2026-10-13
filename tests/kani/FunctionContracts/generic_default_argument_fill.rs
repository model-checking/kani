// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// `proof_for_contract` on a target that omits trailing generic parameters with declared
// defaults. An omitted trailing parameter is now filled from its declared default,
// instantiated with the arguments so far — what rustc does for omitted arguments. These
// all resolve and verify:
//   * a std container default: `Vec<u8>` fills `A = Global`,
//   * sibling discrimination under the fill: `Vec<u16>`'s impl has a distinct
//     postcondition, so each harness only verifies if resolution picks its own impl,
//   * a default referencing an earlier parameter: `Pair<u8>` fills `U = T` as `u8`,
//   * the explicit spelling (`Vec<u16, std::alloc::Global>`) resolves unchanged,
//   * an all-defaulted type written with no argument list: `AllDefault` fills `T = u8`.
// `tests/expected/function-contract/generic_default_missing_required.rs` guards that a
// missing parameter without a default still fails to resolve.

#![feature(allocator_api)]

trait Sum {
    fn total(&self) -> usize;
}

impl Sum for Vec<u8> {
    #[kani::requires(self.len() < 3)]
    #[kani::ensures(|r| *r == self.len())]
    fn total(&self) -> usize {
        self.len()
    }
}

// Distinct postcondition from the `Vec<u8>` impl, so a harness only verifies if
// resolution lands on *this* impl rather than the sibling.
impl Sum for Vec<u16> {
    #[kani::requires(self.len() < 3)]
    #[kani::ensures(|r| *r == self.len() + 1)]
    fn total(&self) -> usize {
        self.len() + 1
    }
}

// `U`'s default references the earlier parameter, so `Pair<u8>` must fill `U = u8`.
struct Pair<T, U = T>(T, U);

impl Sum for Pair<u8> {
    #[kani::ensures(|r| *r == 2)]
    fn total(&self) -> usize {
        2
    }
}

// All parameters defaulted, written with no argument list at all (`AllDefault`, not
// `AllDefault<..>`): the omitted list must still be filled from the default `T = u8`.
struct AllDefault<T = u8>(T);

impl Sum for AllDefault {
    #[kani::ensures(|r| *r == 1)]
    fn total(&self) -> usize {
        1
    }
}

mod verify {
    use super::*;
    // Kani's path resolver does not see the prelude; name `Vec` explicitly.
    use std::vec::Vec;

    #[kani::proof_for_contract(<Vec<u8> as Sum>::total)]
    fn check_vec_default_filled() {
        let v: Vec<u8> = vec![1, 2];
        let _ = v.total();
    }

    #[kani::proof_for_contract(<Vec<u16> as Sum>::total)]
    fn check_vec_sibling_discriminated() {
        let v: Vec<u16> = vec![1];
        let _ = v.total();
    }

    #[kani::proof_for_contract(<Pair<u8> as Sum>::total)]
    fn check_default_from_earlier_param() {
        let p = Pair(1u8, 2u8);
        let _ = p.total();
    }

    #[kani::proof_for_contract(<Vec<u16, std::alloc::Global> as Sum>::total)]
    fn check_explicit_spelling_unchanged() {
        let v: Vec<u16> = vec![1];
        let _ = v.total();
    }

    #[kani::proof_for_contract(<AllDefault as Sum>::total)]
    fn check_all_parameters_defaulted() {
        let a = AllDefault(0u8);
        let _ = a.total();
    }
}
