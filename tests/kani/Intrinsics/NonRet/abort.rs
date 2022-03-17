// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

#![feature(core_intrinsics)]

// Aborts the execution of the process
//
// The current implementation in Rust is to invoke an invalid instruction on
// most platforms. On Unix, the process terminates with a signal like `SIGABRT`,
// `SIGILL`, `SIGTRAP`, `SIGSEGV` or `SIGBUS`.
//
// The documentation mentions that `std::process::abort` is preferred if
// possible: https://doc.rust-lang.org/core/intrinsics/fn.abort.html
// In Kani, `std::process::abort` is identified as a panicking function
#[kani::proof]
fn main() {
    std::intrinsics::abort();
}
