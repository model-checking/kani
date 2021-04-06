// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! this module copies and modifies [rustc_mir::monomorphize]. Please refer to that module for more
//! verbose documentation.
//!
//! A short explanation goes as follows:
//! The rust compiler implements monomorphization as a strategy to handle generics, which expands
//! data and function definitions for each concrete instance as an extra copy. This module implements
//! this strategy, and generates a set of [MonoItem] which represent monomorphized codegen items. The
//! main entry point is [partitioning::collect_and_partition_mono_items].
//!
//! However, the default [partitioning::collect_and_partition_mono_items] doesn't do exactly what we
//! want for a number of reasons:
//! 1. it inserts starter code,
//! 2. it expands debug, panic, and other diagnostic code which is quite irrelevant, since panic, for
//!    example, is replaced by assertion,
//! 3. more importantly, if we do not control the collector, we lose all track of the supported
//!    features because the collected code potentially contains arbitrary rust code.
//!
//! One more elegant way to handle this is to modularize collector and partitioner code, but surely
//! this requires coordination with the rust team and cannot be done in short term.
//!
//! The differences between this module and the original copy so far are:
//! 1. starter code is not collected any more,
//! 2. [hooks::GotocHooks] are injected into the collector and intercept those functions with hooks
//!    applied to them.

mod collector;
pub mod partitioning;
