// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// a helper function that given a pair of integers, returns a valid non-empty
/// range (if possible) or `None`
pub fn create_non_empty_range(a: i32, b: i32) -> Option<std::ops::Range<i32>> {
    let range = if a < b {
        Some(a..b)
    } else if b < a {
        Some(b..a)
    } else {
        None
    };
    assert!(range.is_none() || !range.as_ref().unwrap().is_empty());
    range
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_range1() {
        let r = create_non_empty_range(5, 9);
        assert_eq!(r.unwrap(), 5..9);
    }

    #[test]
    fn test_create_range2() {
        let r = create_non_empty_range(35, 2);
        assert_eq!(r.unwrap(), 2..35);
    }

    #[test]
    fn test_create_range3() {
        let r = create_non_empty_range(-5, -5);
        assert!(r.is_none());
    }
}

#[cfg(kani)]
mod kani_checks {
    use super::*;

    #[kani::proof]
    fn check_range() {
        create_non_empty_range(kani::any(), kani::any());
    }
}
