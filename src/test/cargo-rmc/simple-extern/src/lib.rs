// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
pub mod externs;
pub use externs::external_c_assertion;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        unsafe {
            external_c_assertion(12);
        }
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
        let a: u32 = __nondet();

        if a < 100 {
            unsafe {
                external_c_assertion(a);
            }
        }
    }
}
