// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

//! A constructor defined in another crate and used as a function value (e.g. `.map(Some)`) is a
//! reachable instance whose body `rustc_public::Instance::body()` does not return, since rustc no
//! longer encodes optimized MIR for constructors (c.f. `build_missing_body`). The delayed UB pass
//! (and the points-to analysis it runs) retrieved bodies with `instance.body().unwrap()` and
//! crashed on such instances: https://github.com/model-checking/kani/issues/4852

/// The target search of the delayed UB pass visits every reachable instance, including the
/// `Some` constructor reached through `Option::map`.
#[kani::proof]
fn check_ctor_as_fn_value() {
    let x: u8 = kani::any();
    let v = Some(x).map(Some);
    assert_eq!(v, Some(Some(x)));
}

/// With a delayed UB target (a write through a pointer cast to a type with different padding),
/// the points-to analysis also runs, and follows the call into the `Some` constructor.
#[kani::proof]
fn check_ctor_as_fn_value_with_target() {
    let mut value: u128 = 0;
    let ptr = &mut value as *mut u128 as *mut (u8, u32, u64);
    let x: u8 = kani::any();
    unsafe {
        *ptr = (x, 4, 4);
        // Read the written field only: reading `value` as a whole would read padding.
        let v = Some((*ptr).0).map(Some);
        assert_eq!(v, Some(Some(x)));
    }
}

/// The reproducer from the issue (as a proof harness rather than through `kani autoharness`).
#[kani::proof]
fn check_issue_4852_reproducer() {
    if let Ok(x) = "".parse() {
        assert_eq!(8.1415, x)
    }
}
