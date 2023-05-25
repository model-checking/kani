// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
pub mod pair;
pub use pair::Pair;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

#[cfg(kani)]
mod kani_tests {
    use super::*;

    #[kani::proof]
    fn test_sum() {
        let a: u64 = kani::any();
        let b: u64 = kani::any();
        let p = Pair::new(a, b);
        assert!(p.sum() == a.wrapping_add(b));
    }

    #[test]
    fn _playback_type_checks() {
        kani::concrete_playback_run(vec![], test_sum);
    }
}
