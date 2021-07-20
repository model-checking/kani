// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use super::Irep;

/// A direct implementation of the CBMC serilization format for symbols implemented in
/// https://github.com/diffblue/cbmc/blob/develop/src/util/symbol.h
// TODO: do we want these members to be public?
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Symbol {
    pub typ: Irep,
    pub value: Irep,
    pub location: Irep,
    /// Unique identifier, same as key in symbol table `foo::x`
    pub name: String,
    /// Only used by verilog
    pub module: String,
    /// Local identifier `x`
    pub base_name: String,
    /// Almost always the same as base_name, but with name mangling can be relevant
    pub pretty_name: String,
    /// Currently set to C. Consider creating a "rust" mode and using it in cbmc
    /// https://github.com/model-checking/rmc/issues/1
    pub mode: String,

    // global properties
    pub is_type: bool,
    pub is_macro: bool,
    pub is_exported: bool,
    pub is_input: bool,
    pub is_output: bool,
    pub is_state_var: bool,
    pub is_property: bool,

    // ansi-C properties
    pub is_static_lifetime: bool,
    pub is_thread_local: bool,
    pub is_lvalue: bool,
    pub is_file_local: bool,
    pub is_extern: bool,
    pub is_volatile: bool,
    pub is_parameter: bool,
    pub is_auxiliary: bool,
    pub is_weak: bool,
}
