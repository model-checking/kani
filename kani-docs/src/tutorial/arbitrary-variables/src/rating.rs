// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// ANCHOR: rating_struct
#[derive(Copy, Clone)]
pub struct Rating {
    value: u8,
}

impl Rating {
    pub fn from(value: u8) -> Option<Rating> {
        if value <= 5 { Some(Rating { value }) } else { None }
    }

    pub fn get(&self) -> u8 {
        self.value
    }
}

// ANCHOR_END: rating_struct

#[cfg(rmc)]
mod verification {
    use super::*;

    // ANCHOR: rating_invariant
    unsafe impl rmc::Invariant for Rating {
        fn is_valid(&self) -> bool {
            self.value <= 5
        }
    }
    // ANCHOR_END: rating_invariant

    // ANCHOR: verify_rating
    #[rmc::proof]
    pub fn check_rating() {
        let rating = rmc::any::<Rating>();
        assert!(rating.get() <= 5);
        assert!(Rating::from(rating.get()).is_some());
    }
    // ANCHOR_END: verify_rating
}
