// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::sync::atomic::{AtomicBool, Ordering};

pub trait UserInput {
    fn set_symbol_table_passes(&mut self, passes: Vec<String>);
    fn get_symbol_table_passes(&self) -> Vec<String>;

    fn set_emit_vtable_restrictions(&mut self, restrictions: bool);
    fn get_emit_vtable_restrictions(&self) -> bool;

    fn set_check_assertion_reachability(&mut self, reachability: bool);
    fn get_check_assertion_reachability(&self) -> bool;

    fn set_output_pretty_json(&mut self, pretty_json: bool);
    fn get_output_pretty_json(&self) -> bool;

    fn set_ignore_global_asm(&mut self, global_asm: bool);
    fn get_ignore_global_asm(&self) -> bool;

    #[cfg(feature = "unsound_experiments")]
    fn set_zero_init_vars(&mut self, zero_init: bool);
    #[cfg(feature = "unsound_experiments")]
    fn get_zero_init_vars(&self) -> bool;
}

#[derive(Debug, Default)]
pub struct QueryDb {
    check_assertion_reachability: AtomicBool,
    emit_vtable_restrictions: AtomicBool,
    symbol_table_passes: Vec<String>,
    json_pretty_print: AtomicBool,
    ignore_global_asm: AtomicBool,
    #[cfg(feature = "unsound_experiments")]
    zero_init_vars: AtomicBool,
}

impl UserInput for QueryDb {
    fn set_symbol_table_passes(&mut self, passes: Vec<String>) {
        self.symbol_table_passes = passes;
    }

    fn get_symbol_table_passes(&self) -> Vec<String> {
        self.symbol_table_passes.clone()
    }

    fn set_emit_vtable_restrictions(&mut self, restrictions: bool) {
        self.emit_vtable_restrictions.store(restrictions, Ordering::Relaxed);
    }

    fn get_emit_vtable_restrictions(&self) -> bool {
        self.emit_vtable_restrictions.load(Ordering::Relaxed)
    }

    fn set_check_assertion_reachability(&mut self, reachability: bool) {
        self.check_assertion_reachability.store(reachability, Ordering::Relaxed);
    }

    fn get_check_assertion_reachability(&self) -> bool {
        self.check_assertion_reachability.load(Ordering::Relaxed)
    }

    fn set_output_pretty_json(&mut self, pretty_json: bool) {
        self.json_pretty_print.store(pretty_json, Ordering::Relaxed);
    }

    fn get_output_pretty_json(&self) -> bool {
        self.json_pretty_print.load(Ordering::Relaxed)
    }

    fn set_ignore_global_asm(&mut self, global_asm: bool) {
        self.ignore_global_asm.store(global_asm, Ordering::Relaxed);
    }

    fn get_ignore_global_asm(&self) -> bool {
        self.ignore_global_asm.load(Ordering::Relaxed)
    }

    #[cfg(feature = "unsound_experiments")]
    fn set_zero_init_vars(&mut self, zero_init: bool) {
        self.zero_init_vars.store(zero_init, Ordering::Relaxed);
    }

    #[cfg(feature = "unsound_experiments")]
    fn get_zero_init_vars(&self) -> bool {
        self.zero_init_vars.load(Ordering::Relaxed)
    }
}
