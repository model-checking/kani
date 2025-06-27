// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module hosts a codegen backend that produces low-level borrow calculus
//! (LLBC), which is the format defined by Charon/Aeneas

mod compiler_interface;
mod mir_to_ullbc;

pub use compiler_interface::LlbcCodegenBackend;
