// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// #[kani::requires(assert_valid_ptr(dst) && has_valid_value(dst))]
#[kani::requires(true)]
#[kani::modifies(dst)]
pub unsafe fn replace<T>(dst: *mut T, src: T) -> T {
    std::ptr::replace(dst, src)
}

#[kani::proof_for_contract(replace)]
pub fn check_replace_unit() {
    check_replace_impl::<()>();
}

fn check_replace_impl<T: kani::Arbitrary + Eq + Clone>() {
    let mut dst = T::any();
    let orig = dst.clone();
    let src = T::any();
    let ret = unsafe { replace(&mut dst, src.clone()) };
    assert_eq!(ret, orig);
    assert_eq!(dst, src);
}