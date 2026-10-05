// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// compile-flags: -Zmir-opt-level=2

//! Test projections applied to a subslice bound by a slice pattern. With `-Zmir-opt-level=2`,
//! rustc folds the subslice reference into the place that uses it, e.g. `(*_1)[1:][0 of 1]`,
//! which used to crash Kani. See https://github.com/model-checking/kani/issues/2682.

/// `ConstantIndex` after `Subslice`.
fn tail_first(x: &[u8]) -> Option<u8> {
    match x {
        [_, tail @ ..] if !tail.is_empty() => Some(tail[0]),
        _ => None,
    }
}

/// `ConstantIndex { from_end: true }` after `Subslice`.
fn tail_last(x: &[u8]) -> Option<u8> {
    match x {
        [_, tail @ ..] => match tail {
            [.., last] => Some(*last),
            [] => None,
        },
        [] => None,
    }
}

/// `Index` after `Subslice`.
fn tail_at(x: &[u8], i: usize) -> Option<u8> {
    match x {
        [_, tail @ ..] if i < tail.len() => Some(tail[i]),
        _ => None,
    }
}

/// `Subslice` after `Subslice`.
fn tail_tail_len(x: &[u8]) -> usize {
    match x {
        [_, tail @ ..] => match tail {
            [_, rest @ .., _] => rest.len(),
            _ => 0,
        },
        [] => 0,
    }
}

#[kani::proof]
fn check_subslice_projections() {
    let arr: [u8; 4] = kani::any();
    let len: usize = kani::any_where(|l| *l <= 4);
    let x = &arr[..len];
    let i: usize = kani::any();
    assert_eq!(tail_first(x), if len >= 2 { Some(arr[1]) } else { None });
    assert_eq!(tail_last(x), if len >= 2 { Some(arr[len - 1]) } else { None });
    assert_eq!(tail_at(x, i), if len >= 1 && i < len - 1 { Some(arr[i + 1]) } else { None });
    assert_eq!(tail_tail_len(x), if len >= 3 { len - 3 } else { 0 });
}

/// The reproducer from the issue.
#[kani::proof]
fn check_issue_2682() {
    let x: &[isize] = &[1, 2, 3, 4, 5];
    if !x.is_empty() {
        let el = match x {
            &[1, ref tail @ ..] => &tail[0],
            _ => unreachable!(),
        };
        assert_eq!(*el, 2);
    }
}
