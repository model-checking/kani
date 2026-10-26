// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Testcase for https://github.com/model-checking/kani/issues/5024: reading the
// discriminant of an enum whose niche starts above u64::MAX used to panic the
// compiler.

/// `Option<Signed>` keeps `None` in the tag value -1, that is u128::MAX.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(i128)]
enum Signed {
    A = 0,
    B = i128::MAX,
}

/// `Option<Option<Unsigned>>` keeps its two `None`s in the tag values
/// 2^100 - 2 and 2^100 - 1, a niche of two variants above u64::MAX.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u128)]
enum Unsigned {
    A = 1 << 100,
    B = (1 << 100) + 1,
}

/// No `repr`: rustc's layout alone gives `Option<Outer>` a niche at u128::MAX
/// in a 128-bit tag.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Inner {
    Range { flag: bool, lo: i128, hi: i128 },
    Equals(i128),
    NotEquals(u32),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Outer {
    Numeric { flag: bool, lo: Option<i128>, slack: Option<i128> },
    Wrapped(Inner),
}

#[kani::proof]
fn check_signed_niche_at_u128_max() {
    let some: bool = kani::any();
    let opt = if some { Some(Signed::B) } else { None };
    assert_eq!(opt.is_some(), some);
    assert_eq!(opt == Some(Signed::B), some);
    assert_eq!(opt == Some(Signed::A), false);
    let other: Option<Signed> = if some { None } else { Some(Signed::A) };
    assert_eq!(other.is_none(), some);
}

#[kani::proof]
fn check_unsigned_niche_near_u128_max() {
    let pick: u8 = kani::any();
    let opt: Option<Option<Unsigned>> = match pick {
        0 => None,
        1 => Some(None),
        2 => Some(Some(Unsigned::A)),
        _ => Some(Some(Unsigned::B)),
    };
    assert_eq!(opt.is_none(), pick == 0);
    assert_eq!(opt == Some(None), pick == 1);
    assert_eq!(opt == Some(Some(Unsigned::A)), pick == 2);
    assert_eq!(opt == Some(Some(Unsigned::B)), pick > 2);
}

#[kani::proof]
fn check_layout_niche_at_u128_max() {
    let n: i128 = kani::any();
    let opt = if n > 0 {
        Some(Outer::Numeric { flag: true, lo: Some(n), slack: None })
    } else if n == 0 {
        Some(Outer::Wrapped(Inner::NotEquals(7)))
    } else {
        None
    };
    assert_eq!(opt.is_none(), n < 0);
    match opt {
        Some(Outer::Numeric { lo, slack, .. }) => {
            assert!(n > 0);
            assert_eq!(lo, Some(n));
            assert_eq!(slack, None);
        }
        Some(Outer::Wrapped(inner)) => {
            assert_eq!(n, 0);
            assert_eq!(inner, Inner::NotEquals(7));
        }
        None => assert!(n < 0),
    }
}

/// The tag values Kani writes match rustc's layout.
#[kani::proof]
fn check_niche_values_match_layout() {
    let none: Option<Signed> = std::hint::black_box(None);
    assert_eq!(unsafe { std::mem::transmute::<Option<Signed>, u128>(none) }, u128::MAX);
    let none: Option<Option<Unsigned>> = std::hint::black_box(None);
    let some_none: Option<Option<Unsigned>> = std::hint::black_box(Some(None));
    assert_eq!(
        unsafe { std::mem::transmute::<Option<Option<Unsigned>>, u128>(none) },
        (1 << 100) - 2
    );
    assert_eq!(
        unsafe { std::mem::transmute::<Option<Option<Unsigned>>, u128>(some_none) },
        (1 << 100) - 1
    );
}
