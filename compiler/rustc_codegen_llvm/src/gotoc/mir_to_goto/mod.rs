// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module takes MIR and emits CBMC goto.

pub mod assumptions;
pub mod backend;
pub mod codegen;
pub mod context;
pub mod debug;
pub mod hooks;
pub mod monomorphize;
pub mod stubs;
pub mod utils;
pub use context::GotocCtx;
