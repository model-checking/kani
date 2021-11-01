// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module provides a mechanism which RMC can use to override standard codegen.
//! For example, we the RMC provides pseudo-functions, such as rmc::assume().
//! These functions should not be codegenned as MIR.
//! Instead, we use a "hook" to generate the correct CBMC intrinsic.

mod hooks;
mod stubs;

pub use hooks::{skip_monomorphize, type_and_fn_hooks, GotocHooks, GotocTypeHooks};
