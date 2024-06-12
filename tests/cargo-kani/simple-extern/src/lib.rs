// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
pub mod externs;
pub use externs::external_c_assertion;
// TODO: our reachability analysis does not see through C functions
pub use externs::rust_add1;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        unsafe {
            external_c_assertion(12);
        }
    }
}

#[cfg(kani)]
mod kani_tests {
    use super::*;

    #[kani::proof]
    fn test_sum() {
        let a: u32 = kani::any();

        if a < 100 {
            unsafe {
                external_c_assertion(a);
                rust_add1(a);
            }
        }
    }
}
