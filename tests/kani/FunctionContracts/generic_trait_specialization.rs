// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts -Z stubbing

// Regression for kani#4908.
#![feature(min_specialization)]
#[cfg(kani)]
mod check {
    trait SmallSort {
        fn small_sort<F: FnMut(&u8, &u8) -> bool>(v: &mut [u8], less: &mut F);
    }
    impl<T> SmallSort for T {
        default fn small_sort<F: FnMut(&u8, &u8) -> bool>(v: &mut [u8], _less: &mut F) {
            pair(v);
        }
    }
    impl SmallSort for u8 {
        #[kani::ensures(|_| true)]
        #[kani::modifies(v)]
        fn small_sort<F: FnMut(&u8, &u8) -> bool>(v: &mut [u8], _less: &mut F) {
            pair(v);
        }
    }
    fn pair(v: &mut [u8]) {
        if v.len() == 2 && v[1] < v[0] {
            v.swap(0, 1);
        }
    }
    #[kani::proof_for_contract(<u8 as SmallSort>::small_sort)]
    #[kani::unwind(3)]
    fn check_generic() {
        let mut v: [u8; 2] = kani::any();
        let mut less = |a: &u8, b: &u8| a < b;
        <u8 as SmallSort>::small_sort(&mut v, &mut less);
    }

    #[kani::proof]
    #[kani::should_panic]
    #[kani::stub_verified(<u8 as SmallSort>::small_sort)]
    fn contract_does_not_guarantee_sorting() {
        let mut v = [1u8, 0];
        let mut less = |a: &u8, b: &u8| a < b;
        <u8 as SmallSort>::small_sort(&mut v, &mut less);
        assert!(v[0] <= v[1]);
    }
}
