// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::cmp::Ordering::*;

/// this is interestingly a wrong implementation at
/// https://rosettacode.org/wiki/Binary_search#Rust
fn binary_search_wrong<T: Ord>(arr: &[T], elem: &T) -> Option<usize> {
    let mut size = arr.len();
    let mut base = 0;

    while size > 0 {
        size /= 2;
        let mid = base + size;
        base = match arr[mid].cmp(elem) {
            Less => mid,
            Greater => base,
            Equal => return Some(mid),
        };
    }

    None
}

fn binary_search<T: Ord>(arr: &[T], elem: &T) -> Option<usize> {
    let mut cap = arr.len() - 1;
    let mut low = 0;

    while low < cap {
        let mut mid = (low + cap) / 2;
        if mid == low {
            mid += 1;
        }
        match arr[mid].cmp(elem) {
            Less => low = mid,
            Greater => cap = mid,
            Equal => return Some(mid),
        }
    }

    None
}

fn __nondet<T>() -> T {
    unimplemented!()
}

fn get() -> [i32; 11] {
    [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]
}

fn main() {
    let x = get();
    let y = __nondet();
    if 1 <= y && y <= 11 {
        assert!(binary_search_wrong(&x, &y) == Some(y as usize - 1)); // this fails

        assert!(binary_search(&x, &y) == Some(y as usize - 1));
    }
}
