// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub mod cbmc;
mod logging;
pub use logging::{rmc_debug, rmc_log, rmc_warn, WarningType};
mod mir_to_goto;
pub use mir_to_goto::GotocCodegenBackend;
