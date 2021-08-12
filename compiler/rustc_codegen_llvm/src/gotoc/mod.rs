// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

mod assumptions;
mod backend;
pub mod cbmc;
mod debug;
mod mir_to_goto;
mod utils;
pub use backend::GotocCodegenBackend;
