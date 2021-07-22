// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-verify-fail

#![feature(core_intrinsics)]
use std::intrinsics;
fn main() {
    intrinsics::abort();
}
