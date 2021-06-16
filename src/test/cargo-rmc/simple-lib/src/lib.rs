// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
pub mod pair;
pub use pair::Pair;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

#[cfg(rmc)]
mod rmc_tests {
    use super::*;

    fn __nondet<T>() -> T {
        unimplemented!()
    }
    #[allow(dead_code)]
    #[no_mangle]
    fn test_sum() {
        let a: u64 = __nondet();
        let b: u64 = __nondet();
        let p = Pair::new(a, b);
        assert!(p.sum() == a.wrapping_add(b));
    }
}
