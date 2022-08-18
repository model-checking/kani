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

    fn set_enforce_contracts(&mut self, enforce_contracts: bool);
    fn get_enforce_contracts(&self) -> bool;

    fn set_replace_with_contracts(&mut self, replace_with_contracts: bool);
    fn get_replace_with_contracts(&self) -> bool;
}

#[derive(Debug, Default)]
pub struct QueryDb {
    check_assertion_reachability: AtomicBool,
    emit_vtable_restrictions: AtomicBool,
    symbol_table_passes: Vec<String>,
    json_pretty_print: AtomicBool,
    ignore_global_asm: AtomicBool,
    enforce_contracts: AtomicBool,
    replace_with_contracts: AtomicBool,
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

    fn set_enforce_contracts(&mut self, enforce_contracts: bool) {
        self.enforce_contracts.store(enforce_contracts, Ordering::Relaxed);
    }
    fn get_enforce_contracts(&self) -> bool {
        self.enforce_contracts.load(Ordering::Relaxed)
    }

    fn set_replace_with_contracts(&mut self, replace_with_contracts: bool) {
        self.replace_with_contracts.store(replace_with_contracts, Ordering::Relaxed);
    }

    fn get_replace_with_contracts(&self) -> bool {
        self.replace_with_contracts.load(Ordering::Relaxed)
    }
}
