// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Regression test for https://github.com/model-checking/kani/issues/4757.
//!
//! Under --constructor-args, the assumptions made while generating a harness's inputs can exclude
//! every input: a constructor that never returns a value, or mined conditions that contradict the
//! constructor or each other. Such a harness never reaches the function it verifies and must be
//! reported as a vacuous failure that names the function.

pub mod m {
    // Assert-guarded representation constructor (inlined, with its assertions turned into
    // assumptions) whose assertions no argument satisfies.
    pub struct P {
        v: u8,
    }

    impl P {
        pub unsafe fn new_unchecked(v: u8) -> Self {
            assert!(v > 10);
            assert!(v < 5);
            P { v }
        }

        pub fn boom_p(&self) -> u8 {
            assert!(self.v > 10 && self.v < 5, "always fails");
            self.v
        }
    }

    // Checked public constructor that always returns `None`.
    pub struct Q {
        v: u8,
    }

    impl Q {
        pub fn new(_v: u8) -> Option<Self> {
            None
        }

        pub fn boom_q(&self) -> u8 {
            assert!(self.v > 10 && self.v < 5, "always fails");
            self.v
        }
    }

    // A formatting-trait harness reaches `fmt` through Kani's formatting model, not a direct call.
    impl core::fmt::Display for Q {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            assert!(self.v > 10 && self.v < 5, "always fails");
            f.write_str("Q")
        }
    }

    // Not vacuous: no `Q` can be generated, but the `None` path reaches the function.
    pub fn opt(q: Option<Q>, d: u8) -> u8 {
        match q {
            Some(q) => q.boom_q(),
            None => d,
        }
    }

    // Control: a checked constructor that returns a value for some inputs.
    pub struct R {
        v: u8,
    }

    impl R {
        pub fn new(v: u8) -> Option<Self> {
            if v > 0 { Some(R { v }) } else { None }
        }

        pub fn get(&self) -> u8 {
            assert!(self.v > 0, "R::new rejects zero");
            self.v
        }
    }

    // Satisfiable checked constructor, but the condition mined from `a` and `b` (`v < 5`),
    // assumed once the constructor returns, excludes every value it returns.
    pub struct S {
        v: u8,
    }

    impl S {
        pub fn new(v: u8) -> Option<Self> {
            if v > 10 { Some(S { v }) } else { None }
        }

        pub fn a(&self) -> u8 {
            assert!(self.v < 5, "a needs v < 5");
            self.v
        }

        pub fn b(&self) -> u8 {
            assert!(self.v < 5, "b needs v < 5");
            self.v / 2
        }
    }

    // The same with a satisfiable assert-guarded constructor.
    pub struct T {
        v: u8,
    }

    impl T {
        pub unsafe fn new_unchecked(v: u8) -> Self {
            assert!(v > 10);
            T { v }
        }

        pub fn c(&self) -> u8 {
            assert!(self.v < 5, "c needs v < 5");
            self.v
        }

        pub fn d(&self) -> u8 {
            assert!(self.v < 5, "d needs v < 5");
            self.v / 2
        }
    }

    // No constructor, and the conditions mined from `a` and `b` (`v < 5`) and from `c` and `d`
    // (`v > 10`) contradict each other.
    pub struct V {
        v: u8,
    }

    impl V {
        pub fn a(&self) -> u8 {
            assert!(self.v < 5, "a");
            self.v
        }

        pub fn b(&self) -> u8 {
            assert!(self.v < 5, "b");
            self.v / 2
        }

        pub fn c(&self) -> u8 {
            assert!(self.v > 10, "c");
            self.v
        }

        pub fn d(&self) -> u8 {
            assert!(self.v > 10, "d");
            self.v / 3
        }
    }
}
