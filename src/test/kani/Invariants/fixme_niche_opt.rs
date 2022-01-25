// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// This testcase is currently failing with the following error:
///
/// [assertion.2] Reached assignment statement with unequal types Pointer { typ:
/// StructTag("tag-Unit") } Pointer { typ: StructTag("tag-_3943305294634710273") }: FAILURE
///
/// If you run:
/// ```
/// RUSTFLAGS="--cfg=ok" kani fixme_niche_opt.rs
/// ```
/// This test will succeed.
///
/// Issue: https://github.com/model-checking/kani/issues/729

enum Error {
    Error1,
    Error2,
}

/// This version fails.
#[cfg(not(ok))]
fn to_option<T: Copy, E>(result: &Result<T, E>) -> Option<T> {
    if let Ok(v) = result { Some(*v) } else { None }
}

/// This version succeeds.
#[cfg(ok)]
fn to_option<T: Copy, E>(result: &Result<T, E>) -> Option<T> {
    if let Ok(v) = *result { Some(v) } else { None }
}

fn main() {
    let result: Result<(), Error> = Ok(());
    assert!(to_option(&result).is_some());
}
