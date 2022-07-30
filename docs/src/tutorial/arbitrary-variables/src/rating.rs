// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// ANCHOR: rating_struct
#[derive(Copy, Clone)]
enum Rating {
    One,
    Two,
    Three,
}

impl Rating {
    fn as_int(&self) -> u8 {
        match self {
            Rating::One => 1,
            Rating::Two => 2,
            Rating::Three => 3,
        }
    }
}
// ANCHOR_END: rating_struct

#[cfg(kani)]
mod verification {
    use super::*;

    // ANCHOR: rating_invariant
    impl kani::Arbitrary for Rating {
        fn any() -> Self {
            match kani::any::<u8>() {
                0 => Rating::One,
                1 => Rating::Two,
                _ => Rating::Three,
            }
        }
    }
    // ANCHOR_END: rating_invariant

    // ANCHOR: verify_rating
    #[kani::proof]
    pub fn check_rating() {
        let rating: Rating = kani::any();
        assert!((1..=3).contains(&rating.as_int()));
    }
    // ANCHOR_END: verify_rating
}
