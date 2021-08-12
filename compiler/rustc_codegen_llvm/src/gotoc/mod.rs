// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

mod assumptions;
mod backend;
pub mod cbmc;
mod debug;
mod hooks;
mod mir_to_goto;
pub mod stubs;
mod utils;
pub use backend::GotocCodegenBackend;
