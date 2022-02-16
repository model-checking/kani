// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::sync::atomic::{AtomicBool, Ordering};

pub trait UserInput {
    fn set_symbol_table_passes(&mut self, passes: Vec<String>);
    fn get_symbol_table_passes(&self) -> Vec<String>;

    fn set_emit_vtable_restrictions(&mut self, restrictions: bool);
    fn get_emit_vtable_restrictions(&self) -> bool;

    fn set_check_assertion_reachability(&mut self, reachability: bool);
    fn get_check_assertion_reachability(&self) -> bool;
}

#[derive(Debug, Default)]
pub struct QueryDb {
    check_assertion_reachability: AtomicBool,
    emit_vtable_restrictions: AtomicBool,
    symbol_table_passes: Vec<String>,
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
}
