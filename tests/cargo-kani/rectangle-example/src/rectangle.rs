// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// ANCHOR: rectangle_definition
#[derive(Debug, Copy, Clone)]
struct Rectangle {
    width: u64,
    height: u64,
}

impl Rectangle {
    fn can_hold(&self, other: &Rectangle) -> bool {
        self.width > other.width && self.height > other.height
    }
}
// ANCHOR_END: rectangle_definition

impl Rectangle {
    // ANCHOR: stretch_definition
    fn stretch(&self, factor: u64) -> Option<Self> {
        let w = self.width.checked_mul(factor)?;
        let h = self.height.checked_mul(factor)?;
        Some(Rectangle { width: w, height: h })
    }
    // ANCHOR_END: stretch_definition
}

#[cfg(test)]
mod tests {
    use super::*;

    // ANCHOR: stretch_unit_test
    #[test]
    fn stretched_rectangle_can_hold_original() {
        let original = Rectangle { width: 8, height: 7 };
        let factor = 2;
        let larger = original.stretch(factor);
        assert!(larger.unwrap().can_hold(&original));
    }
    // ANCHOR_END: stretch_unit_test
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::num::u64;
    use proptest::prelude::*;

    proptest! {
        // ANCHOR: stretch_proptest
        #[test]
        fn stretched_rectangle_can_hold_original(width in u64::ANY,
            height in u64::ANY,
            factor in u64::ANY) {
            let original = Rectangle {
                width: width,
                height: height,
            };
            if let Some(larger) = original.stretch(factor) {
                assert!(larger.can_hold(&original));
            }
        }
        // ANCHOR_END: stretch_proptest
    }
}

#[cfg(kani)]
mod verification {
    use super::*;

    // ANCHOR: stretch_kani
    #[kani::proof]
    pub fn stretched_rectangle_can_hold_original() {
        let original = Rectangle { width: kani::any(), height: kani::any() };
        let factor = kani::any();
        if let Some(larger) = original.stretch(factor) {
            assert!(larger.can_hold(&original));
        }
    }
    // ANCHOR_END: stretch_kani

    // ANCHOR: stretch_kani_fixed
    #[kani::proof]
    pub fn stretched_rectangle_can_hold_original_fixed() {
        let original = Rectangle { width: kani::any(), height: kani::any() };
        let factor = kani::any();
        kani::assume(0 != original.width);
        kani::assume(0 != original.height);
        kani::assume(1 < factor);
        if let Some(larger) = original.stretch(factor) {
            assert!(larger.can_hold(&original));
        }
    }
    // ANCHOR_END: stretch_kani_fixed
}
