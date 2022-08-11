
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

#![allow(unreachable_code)]
mod test {
    proptest! {
        #[test]
        fn the_test(_ in 0u32..100) { panic!() }
    }
}
