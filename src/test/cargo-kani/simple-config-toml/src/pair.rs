// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
pub struct Pair(pub u64, pub u64);

impl Pair {
    pub fn new(a: u64, b: u64) -> Self {
        Pair(a, b)
    }
    pub fn sum(&self) -> u64 {
        self.0.wrapping_add(self.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_plus_two() {
        let p = Pair::new(1, 2);
        assert_eq!(p.sum(), 3);
    }
}

#[cfg(rmc)]
mod rmc_tests {
    use super::*;
    #[allow(dead_code)]
    #[no_mangle]
    fn test_one_plus_two() {
        let p = Pair::new(1, 2);
        assert!(p.sum() == 3);
    }
}
