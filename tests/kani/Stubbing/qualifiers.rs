// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness harness -Z stubbing
//
//! This tests resolving stubs with the path qualifiers `self`, `super`, and
//! `crate`.

fn magic_number() -> u32 {
    10
}

mod mod1 {
    fn magic_number() -> u32 {
        11
    }

    mod mod2 {
        fn magic_number() -> u32 {
            12
        }

        #[kani::proof]
        #[kani::stub(f1, crate::magic_number)]
        #[kani::stub(f2, super::super::magic_number)]
        #[kani::stub(f3, self::super::super::magic_number)]
        #[kani::stub(g1, crate::mod1::magic_number)]
        #[kani::stub(g2, super::magic_number)]
        #[kani::stub(g3, self::super::magic_number)]
        #[kani::stub(h1, crate::mod1::mod2::magic_number)]
        #[kani::stub(h2, super::mod2::magic_number)]
        #[kani::stub(h3, self::magic_number)]
        fn harness() {
            assert_eq!(f1(), 10);
            assert_eq!(f2(), 10);
            assert_eq!(f3(), 10);

            assert_eq!(g1(), 11);
            assert_eq!(g2(), 11);
            assert_eq!(g3(), 11);

            assert_eq!(h1(), 12);
            assert_eq!(h2(), 12);
            assert_eq!(h3(), 12);
        }

        fn f1() -> u32 {
            0
        }

        fn f2() -> u32 {
            0
        }

        fn f3() -> u32 {
            0
        }

        fn g1() -> u32 {
            0
        }

        fn g2() -> u32 {
            0
        }

        fn g3() -> u32 {
            0
        }

        fn h1() -> u32 {
            0
        }

        fn h2() -> u32 {
            0
        }

        fn h3() -> u32 {
            0
        }
    }
}
