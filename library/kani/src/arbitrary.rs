// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module implements `Arbitrary` for some `core` types, and defines the models the compiler
//! uses for closure arguments. The implementations for `alloc` types are in `kani_core`.

use crate::Arbitrary;

impl Arbitrary for std::time::Duration {
    fn any() -> Self {
        const NANOS_PER_SEC: u32 = 1_000_000_000;
        let nanos = u32::any();
        crate::assume(nanos < NANOS_PER_SEC);
        std::time::Duration::new(u64::any(), nanos)
    }
}

impl Arbitrary for std::char::EscapeUnicode {
    fn any() -> Self {
        // Generate any state reachable by consuming a freshly constructed
        // EscapeUnicode iterator from the front.
        let mut escape = char::any().escape_unicode();
        let len = escape.len();

        let front = usize::from(u8::any());
        crate::assume(front <= len);

        if front >= 1 {
            let _ = escape.next();
        }
        if front >= 2 {
            let _ = escape.next();
        }
        if front >= 3 {
            let _ = escape.next();
        }
        if front >= 4 {
            let _ = escape.next();
        }
        if front >= 5 {
            let _ = escape.next();
        }
        if front >= 6 {
            let _ = escape.next();
        }
        if front >= 7 {
            let _ = escape.next();
        }
        if front >= 8 {
            let _ = escape.next();
        }
        if front >= 9 {
            let _ = escape.next();
        }
        if front >= 10 {
            let _ = escape.next();
        }

        escape
    }
}

/// Nondeterministic functions for instantiating `Fn`/`FnMut`/`FnOnce`-bounded type
/// parameters of automatic harnesses: the parameter is instantiated with the *function
/// item type* of the matching-arity model below (function items implement all three `Fn`
/// traits and are zero-sized, so generating the value is trivial). Each call returns a
/// fresh nondeterministic value, which over-approximates the behavior of every real
/// closure with that signature (including stateful `FnMut` closures); verifying the
/// harness against this instantiation therefore covers the function-under-test's own code
/// for any closure behavior.
///
/// These models are *optional* (c.f. `KaniModel::is_optional`).
#[kanitool::fn_marker = "NondetFn0Model"]
#[inline(never)]
#[doc(hidden)]
pub fn nondet_fn0<R: Arbitrary>() -> R {
    crate::any()
}

#[kanitool::fn_marker = "NondetFn1Model"]
#[inline(never)]
#[doc(hidden)]
pub fn nondet_fn1<A, R: Arbitrary>(_a: A) -> R {
    crate::any()
}

#[kanitool::fn_marker = "NondetFn1RefModel"]
#[inline(never)]
#[doc(hidden)]
/// Region-polymorphic: the fn item's late-bound lifetime lets it satisfy HRTB bounds
/// like `for<'a> Fn(&'a T) -> R` that the early-bound by-value models cannot.
pub fn nondet_fn1_ref<T, R: Arbitrary>(_a: &T) -> R {
    crate::any()
}

#[kanitool::fn_marker = "NondetFn2RefRefModel"]
#[inline(never)]
#[doc(hidden)]
pub fn nondet_fn2_ref_ref<A, B, R: Arbitrary>(_a: &A, _b: &B) -> R {
    crate::any()
}

#[kanitool::fn_marker = "NondetFn2RefValModel"]
#[inline(never)]
#[doc(hidden)]
pub fn nondet_fn2_ref_val<A, B, R: Arbitrary>(_a: &A, _b: B) -> R {
    crate::any()
}

#[kanitool::fn_marker = "NondetFn2ValRefModel"]
#[inline(never)]
#[doc(hidden)]
pub fn nondet_fn2_val_ref<A, B, R: Arbitrary>(_a: A, _b: &B) -> R {
    crate::any()
}

#[kanitool::fn_marker = "NondetFn2Model"]
#[inline(never)]
#[doc(hidden)]
pub fn nondet_fn2<A, B, R: Arbitrary>(_a: A, _b: B) -> R {
    crate::any()
}

#[kanitool::fn_marker = "NondetFn3Model"]
#[inline(never)]
#[doc(hidden)]
pub fn nondet_fn3<A, B, C, R: Arbitrary>(_a: A, _b: B, _c: C) -> R {
    crate::any()
}
