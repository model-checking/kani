// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(bool_to_option)]
#![feature(const_cstr_unchecked)]
#![feature(crate_visibility_modifier)]
#![feature(extern_types)]
#![feature(in_band_lifetimes)]
#![feature(iter_zip)]
#![feature(nll)]
#![recursion_limit = "256"]
#![feature(destructuring_assignment)]
#![feature(box_patterns)]
#![feature(once_cell)]

pub mod cbmc;
mod mir_to_goto;
pub use mir_to_goto::GotocCodegenBackend;
