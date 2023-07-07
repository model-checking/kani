// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Try to verify HashSet basic behavior.

use std::collections::{hash_map::RandomState, HashSet};
use std::mem::{size_of, size_of_val, transmute};

fn concrete_state() -> RandomState {
    let keys: [u64; 2] = [0, 0];
    assert_eq!(size_of_val(&keys), size_of::<RandomState>());
    unsafe { transmute(keys) }
}

#[kani::proof]
#[kani::stub(RandomState::new, concrete_state)]
#[kani::unwind(1)]
#[kani::solver(kissat)]
fn check_insert_default() {
    let mut set: HashSet<i8> = HashSet::default();
    let first = 10;
    set.insert(first);
}

// too slow so don't run in the regression for now
#[cfg(slow)]
mod slow {
    #[kani::proof]
    #[kani::stub(RandomState::new, concrete_state)]
    #[kani::unwind(3)]
    fn check_insert_default() {
        let mut set: HashSet<i8> = HashSet::default();
        let first = kani::any();
        set.insert(first);

        let second = kani::any();
        set.insert(second);

        if first == second { assert_eq!(set.len(), 1) } else { assert_eq!(set.len(), 2) }
    }

    #[kani::proof]
    #[kani::stub(RandomState::new, concrete_state)]
    #[kani::unwind(1)]
    fn check_contains() {
        let first = kani::any();
        let mut set: HashSet<i8> = HashSet::from([first]);
        assert!(set.contains(&first));
    }

    #[kani::proof]
    #[kani::stub(RandomState::new, concrete_state)]
    #[kani::unwind(1)]
    #[cfg(slow)]
    fn check_contains_str() {
        let mut set = HashSet::from(["str"]);
        assert!(set.contains(&"str"));
    }
}
