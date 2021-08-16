// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module takes MIR and emits CBMC goto.

mod codegen;
mod compiler_interface;
mod context;
mod monomorphize;
mod overrides;
mod utils;

pub use compiler_interface::GotocCodegenBackend;
pub use context::GotocCtx;
