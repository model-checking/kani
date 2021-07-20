// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains the structures used for symbol table transformations.

mod identity_transformer;
mod passes;
mod transformer;

pub use passes::do_passes;
use transformer::Transformer;
