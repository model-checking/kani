// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Test that Kani can correctly handle match patterns joined with the `|` operator.
//! It contains two equivalent methods that only differ by grouping march patterns.
//! Kani used to only be able to verify one as reported in:
//! <https://github.com/model-checking/kani/issues/3432>

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(kani, derive(kani::Arbitrary))]
pub enum AbstractInt {
    Bottom = 0,
    Zero = 1,
    Top = 2,
}

impl AbstractInt {
    /// Code with exhausive match expression where each arm contains one pattern.
    pub fn merge(self, other: Self) -> Self {
        use AbstractInt::*;
        match (self, other) {
            (Bottom, x) => x,
            (x, Bottom) => x,
            (Zero, Zero) => Zero,
            (Top, _) => Top,
            (_, Top) => Top,
        }
    }

    /// Code with exhausive match expression where an arm may contain multiple patterns.
    pub fn merge_joined(self, other: Self) -> Self {
        use AbstractInt::*;
        match (self, other) {
            (Bottom, x) | (x, Bottom) => x,
            (Zero, Zero) => Zero,
            (Top, _) | (_, Top) => Top,
        }
    }
}

#[cfg(kani)]
mod test {
    use super::*;

    #[kani::proof]
    fn merge_with_bottom() {
        let x: AbstractInt = kani::any();
        assert!(x.merge(AbstractInt::Bottom) == x);
        assert!(AbstractInt::Bottom.merge(x) == x)
    }

    #[kani::proof]
    fn check_equivalence() {
        let x: AbstractInt = kani::any();
        let y: AbstractInt = kani::any();
        assert_eq!(x.merge(y), x.merge_joined(y));
    }
}
