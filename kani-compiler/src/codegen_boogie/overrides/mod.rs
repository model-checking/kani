// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module provides a mechanism which Kani can use to override standard codegen.
//! For example, we the Kani provides pseudo-functions, such as kani::assume().
//! These functions should not be codegenned as MIR.
//! Instead, we use a "hook" to generate the correct code.

mod hooks;

pub use hooks::{fn_hooks, BoogieHooks};
