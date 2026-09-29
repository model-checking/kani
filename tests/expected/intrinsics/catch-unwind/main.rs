// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The `catch_unwind` intrinsic is not supported yet (see
//! <https://github.com/model-checking/kani/issues/267>), but reaching it should report an
//! unsupported construct failure, not crash the compiler. Kani used to ICE on a stale
//! signature assertion after the intrinsic's return type changed from `i32` to `bool`.
//! See <https://github.com/model-checking/kani/issues/4813>.
use std::panic;

#[kani::proof]
fn check_catch_unwind() {
    let result = panic::catch_unwind(|| 42);
    assert!(result.is_ok());
}
