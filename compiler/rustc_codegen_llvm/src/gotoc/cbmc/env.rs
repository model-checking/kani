// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Find mod.rs for centralized documentation
//!
//! This file contains set up code for CBMC symbol table. It modifies a default table based on
//! the current compilation instance's session information.
//!
//! c.f. CBMC code [src/ansi-c/ansi_c_internal_additions.cpp].
//! One possible invocation of this insertion in CBMC can be found in [ansi_c_languaget::parse].

use super::goto_program::{Expr, Location, Symbol, Type};
use super::MachineModel;
use num::bigint::BigInt;
fn int_constant<T>(name: &str, value: T) -> Symbol
where
    T: Into<BigInt>,
{
    Symbol::constant(name, name, name, Expr::int_constant(value, Type::c_int()), Location::none())
}

fn int_constant_from_bool(name: &str, value: bool) -> Symbol {
    Symbol::constant(
        name,
        name,
        name,
        Expr::int_constant(if value { 1 } else { 0 }, Type::c_int()),
        Location::none(),
    )
}

fn string_constant(name: &str, value: &str) -> Symbol {
    Symbol::constant(name, name, name, Expr::string_constant(value), Location::none())
}

pub fn machine_model_symbols(mm: &MachineModel) -> Vec<Symbol> {
    vec![
        string_constant("__CPROVER_architecture_arch", mm.architecture()),
        int_constant_from_bool("__CPROVER_architecture_NULL_is_zero", mm.null_is_zero()),
        int_constant("__CPROVER_architecture_alignment", mm.alignment()),
        int_constant("__CPROVER_architecture_bool_width", mm.bool_width()),
        int_constant_from_bool("__CPROVER_architecture_char_is_unsigned", mm.char_is_unsigned()),
        int_constant("__CPROVER_architecture_char_width", mm.char_width()),
        int_constant("__CPROVER_architecture_double_width", mm.double_width()),
        // c.f. https://github.com/diffblue/cbmc/blob/develop/src/util/config.h
        // the numbers are from endiannesst
        int_constant("__CPROVER_architecture_endianness", if mm.is_big_endian() { 2 } else { 1 }),
        int_constant("__CPROVER_architecture_int_width", mm.int_width()),
        int_constant("__CPROVER_architecture_long_double_width", mm.long_double_width()),
        int_constant("__CPROVER_architecture_long_int_width", mm.long_int_width()),
        int_constant("__CPROVER_architecture_long_long_int_width", mm.long_long_int_width()),
        int_constant("__CPROVER_architecture_memory_operand_size", mm.memory_operand_size()),
        int_constant("__CPROVER_architecture_pointer_width", mm.pointer_width()),
        int_constant("__CPROVER_architecture_short_int_width", mm.short_int_width()),
        int_constant("__CPROVER_architecture_single_width", mm.single_width()),
        int_constant_from_bool(
            "__CPROVER_architecture_wchar_t_is_unsigned",
            mm.wchar_t_is_unsigned(),
        ),
        int_constant("__CPROVER_architecture_wchar_t_width", mm.wchar_t_width()),
        int_constant("__CPROVER_architecture_word_size", mm.word_size()),
        int_constant("__CPROVER_rounding_mode", mm.rounding_mode()),
    ]
}

pub fn additional_env_symbols() -> Vec<Symbol> {
    vec![
        Symbol::builtin_function("__CPROVER_initialize", vec![], Type::empty()),
        // https://github.com/diffblue/cbmc/blob/b26d3479679574c6c179f911b488a314bc2f1085/src/util/config.h#L214
        int_constant("__CPROVER_malloc_failure_mode_assert_then_assume", 2),
        int_constant("__CPROVER_malloc_failure_mode_return_null", 1),
        Symbol::typedef("__CPROVER_size_t", "__CPROVER_size_t", Type::size_t(), Location::none()),
        Symbol::variable(
            "__CPROVER_memory".to_string(),
            "__CPROVER_memory".to_string(),
            Type::unsigned_int(8).infinite_array_of(),
            Location::none(),
        )
        .with_is_extern(true)
        .with_is_static_lifetime(true)
        .with_is_thread_local(false),
    ]
}
