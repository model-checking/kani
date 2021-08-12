// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

mod assumptions;
mod backend;
mod block;
pub mod cbmc;
mod debug;
mod function;
mod hooks;
mod intrinsic;
mod mir_to_goto;
mod operand;
mod place;
mod rvalue;
mod statement;
mod static_var;
pub mod stubs;
mod typ;
mod utils;
pub use backend::GotocCodegenBackend;
